//! ADI Audio Plugin
//!
//! Provides CLI commands for audio processing:
//! - info: Analyze audio file
//! - preset: Apply processing preset (web-sfx, podcast, music-master, etc.)
//! - presets: List available presets
//! - process: Apply mastering chain (highpass, EQ, compression, normalize)
//! - highpass: Apply high-pass filter
//! - eq: Apply parametric EQ
//! - compress: Apply dynamic compression
//! - normalize: Normalize to target loudness
//! - convert: Convert between formats

use abi_stable::std_types::{ROption, RResult, RStr, RString, RVec};
use lib_plugin_abi::{
    PluginContext, PluginError, PluginInfo, PluginVTable, ServiceDescriptor, ServiceError,
    ServiceHandle, ServiceMethod, ServiceVTable, ServiceVersion,
};

use adi_audio_core::{
    apply_preset_with_stats, Compressor, CompressorSettings, EqBand, EqBandType, HighPassFilter,
    Limiter, LoudnessAnalyzer, Normalizer, ParametricEq, Preset, WavReader, WavWriter,
};
use serde_json::json;
use std::ffi::c_void;
use std::path::Path;
use std::process::Command;

/// Service ID for CLI commands
const SERVICE_CLI: &str = "adi.audio.cli";

// === Plugin VTable Implementation ===

extern "C" fn plugin_info() -> PluginInfo {
    PluginInfo::new("adi.audio", "ADI Audio", env!("CARGO_PKG_VERSION"), "core")
        .with_author("ADI Team")
        .with_description("Audio processing: WAV I/O, filters, EQ, compression, normalization")
        .with_min_host_version("0.8.0")
}

extern "C" fn plugin_init(ctx: *mut PluginContext) -> i32 {
    unsafe {
        let host = (*ctx).host();

        // Register CLI service
        let cli_descriptor =
            ServiceDescriptor::new(SERVICE_CLI, ServiceVersion::new(1, 0, 0), "adi.audio")
                .with_description("CLI commands for audio processing");

        let cli_handle = ServiceHandle::new(
            SERVICE_CLI,
            ctx as *const c_void,
            &CLI_SERVICE_VTABLE as *const ServiceVTable,
        );

        if let Err(code) = host.register_svc(cli_descriptor, cli_handle) {
            host.error(&format!("Failed to register CLI service: {}", code));
            return code;
        }

        host.info("ADI Audio plugin initialized");
    }
    0
}

extern "C" fn plugin_cleanup(_ctx: *mut PluginContext) {}

extern "C" fn handle_message(
    _ctx: *mut PluginContext,
    msg_type: RStr<'_>,
    _msg_data: RStr<'_>,
) -> RResult<RString, PluginError> {
    match msg_type.as_str() {
        "ping" => RResult::ROk(RString::from("pong")),
        _ => RResult::RErr(PluginError::new(
            -1,
            format!("Unknown message type: {}", msg_type.as_str()),
        )),
    }
}

// === Plugin Entry Point ===

static PLUGIN_VTABLE: PluginVTable = PluginVTable {
    info: plugin_info,
    init: plugin_init,
    update: ROption::RNone,
    cleanup: plugin_cleanup,
    handle_message: ROption::RSome(handle_message),
};

#[no_mangle]
pub extern "C" fn plugin_entry() -> *const PluginVTable {
    &PLUGIN_VTABLE
}

// === CLI Service VTable ===

static CLI_SERVICE_VTABLE: ServiceVTable = ServiceVTable {
    invoke: cli_invoke,
    list_methods: cli_list_methods,
};

extern "C" fn cli_invoke(
    _handle: *const c_void,
    method: RStr<'_>,
    args: RStr<'_>,
) -> RResult<RString, ServiceError> {
    match method.as_str() {
        "run_command" => {
            let result = run_cli_command(args.as_str());
            match result {
                Ok(output) => RResult::ROk(RString::from(output)),
                Err(e) => RResult::RErr(ServiceError::invocation_error(e)),
            }
        }
        "list_commands" => {
            let commands = json!([
                {"name": "info", "description": "Analyze audio file", "usage": "info <file>"},
                {"name": "preset", "description": "Apply processing preset", "usage": "preset <input> <output.wav> --preset <name>"},
                {"name": "presets", "description": "List available presets", "usage": "presets"},
                {"name": "process", "description": "Apply full mastering chain", "usage": "process <input> <output.wav> [options]"},
                {"name": "highpass", "description": "Apply high-pass filter", "usage": "highpass <input> <output.wav> --freq <hz>"},
                {"name": "eq", "description": "Apply parametric EQ", "usage": "eq <input> <output.wav> [--preset <voice|master>] [--band <type:freq:gain:q>]"},
                {"name": "compress", "description": "Apply dynamic compression", "usage": "compress <input> <output.wav> [--preset <gentle|vocal|aggressive|limiter>]"},
                {"name": "normalize", "description": "Normalize to target loudness", "usage": "normalize <input> <output.wav> [--lufs <target>] [--peak <db>]"},
                {"name": "convert", "description": "Convert audio format", "usage": "convert <input> <output.wav> [--bits <16|24|32>] [--float]"}
            ]);
            RResult::ROk(RString::from(
                serde_json::to_string(&commands).unwrap_or_default(),
            ))
        }
        _ => RResult::RErr(ServiceError::method_not_found(method.as_str())),
    }
}

extern "C" fn cli_list_methods(_handle: *const c_void) -> RVec<ServiceMethod> {
    vec![
        ServiceMethod::new("run_command").with_description("Run a CLI command"),
        ServiceMethod::new("list_commands").with_description("List available commands"),
    ]
    .into_iter()
    .collect()
}

fn run_cli_command(context_json: &str) -> Result<String, String> {
    let context: serde_json::Value =
        serde_json::from_str(context_json).map_err(|e| format!("Invalid context: {}", e))?;

    let args: Vec<String> = context
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let subcommand = args.first().map(|s| s.as_str()).unwrap_or("");
    let cmd_args: Vec<&str> = args.iter().skip(1).map(|s| s.as_str()).collect();

    match subcommand {
        "info" => cmd_info(&cmd_args),
        "preset" => cmd_preset(&cmd_args),
        "presets" => cmd_presets(),
        "process" => cmd_process(&cmd_args),
        "highpass" => cmd_highpass(&cmd_args),
        "eq" => cmd_eq(&cmd_args),
        "compress" => cmd_compress(&cmd_args),
        "normalize" => cmd_normalize(&cmd_args),
        "convert" => cmd_convert(&cmd_args),
        "" => Ok(get_help()),
        _ => Err(format!("Unknown command: {}", subcommand)),
    }
}

fn get_help() -> String {
    r#"ADI Audio - Audio processing toolkit

Commands:
  info       Analyze audio file (duration, sample rate, loudness, peak)
  preset     Apply processing preset (web-sfx, podcast, music-master, etc.)
  presets    List all available presets
  process    Apply full mastering chain (highpass, EQ, compression, normalize)
  highpass   Apply high-pass filter to remove low frequencies
  eq         Apply parametric EQ with bands or presets
  compress   Apply dynamic range compression
  normalize  Normalize to target loudness (LUFS) or peak level
  convert    Convert audio format (bit depth, sample format)

Supported input formats: WAV, MP3, FLAC, OGG, M4A (requires ffmpeg)
Output format: WAV (16/24/32-bit PCM or 32-bit float)

Usage: adi audio <command> [args]

Examples:
  adi audio info recording.mp3
  adi audio preset sound.mp3 output.wav --preset web-sfx
  adi audio presets
  adi audio process input.wav output.wav --highpass 80 --compress gentle --lufs -14"#
        .to_string()
}

// === Audio Loading (supports WAV, MP3, FLAC, etc. via ffmpeg) ===

/// Load audio from any supported format (WAV native, others via ffmpeg)
fn load_audio(input_path: &str) -> Result<adi_audio_core::AudioBuffer, String> {
    let path = Path::new(input_path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "wav" => WavReader::read(input_path).map_err(|e| format!("Failed to read WAV: {}", e)),
        "mp3" | "flac" | "ogg" | "m4a" | "aac" | "opus" | "wma" => convert_with_ffmpeg(input_path),
        _ => {
            // Try WAV first, then ffmpeg
            WavReader::read(input_path)
                .or_else(|_| convert_with_ffmpeg(input_path))
                .map_err(|e| format!("Unsupported format or failed to read: {}", e))
        }
    }
}

/// Convert audio file to WAV using ffmpeg and read it
fn convert_with_ffmpeg(input_path: &str) -> Result<adi_audio_core::AudioBuffer, String> {
    // Check if ffmpeg is available
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        return Err(
            "ffmpeg not found. Install ffmpeg to process MP3/FLAC/etc files.\n\
                    macOS: brew install ffmpeg\n\
                    Ubuntu: apt install ffmpeg"
                .to_string(),
        );
    }

    // Create temp file for WAV output
    let temp_dir = std::env::temp_dir();
    let temp_wav = temp_dir.join(format!("adi_audio_{}.wav", std::process::id()));
    let temp_wav_str = temp_wav.to_string_lossy();

    // Convert to WAV (44.1kHz, 16-bit, stereo)
    let output = Command::new("ffmpeg")
        .args([
            "-y", // Overwrite output
            "-i",
            input_path,
            "-ar",
            "44100", // Sample rate
            "-ac",
            "2", // Stereo
            "-sample_fmt",
            "s16", // 16-bit
            "-f",
            "wav",
            &temp_wav_str,
        ])
        .output()
        .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_wav);
        return Err(format!("ffmpeg conversion failed: {}", stderr));
    }

    // Read the converted WAV
    let buffer = WavReader::read(temp_wav.as_path())
        .map_err(|e| format!("Failed to read converted audio: {}", e));

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_wav);

    buffer
}

/// Write audio to output file, auto-detecting format from extension
/// Supports: WAV (native), MP3/OGG/FLAC/M4A (via ffmpeg)
fn write_audio(output_path: &str, buffer: &adi_audio_core::AudioBuffer) -> Result<(), String> {
    let path = Path::new(output_path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "wav" => {
            WavWriter::write(output_path, buffer).map_err(|e| format!("Failed to write WAV: {}", e))
        }
        "mp3" | "ogg" | "flac" | "m4a" | "aac" | "opus" => {
            convert_to_format(output_path, buffer, &ext)
        }
        _ => {
            // Default to WAV
            WavWriter::write(output_path, buffer).map_err(|e| format!("Failed to write: {}", e))
        }
    }
}

/// Convert audio buffer to compressed format using ffmpeg
fn convert_to_format(
    output_path: &str,
    buffer: &adi_audio_core::AudioBuffer,
    format: &str,
) -> Result<(), String> {
    // Check if ffmpeg is available
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        return Err(format!(
            "ffmpeg not found. Install ffmpeg to export to {} format.\n\
             macOS: brew install ffmpeg\n\
             Ubuntu: apt install ffmpeg",
            format
        ));
    }

    // Write temp WAV
    let temp_dir = std::env::temp_dir();
    let temp_wav = temp_dir.join(format!("adi_audio_out_{}.wav", std::process::id()));

    WavWriter::write(&temp_wav, buffer).map_err(|e| format!("Failed to write temp WAV: {}", e))?;

    // Determine ffmpeg codec and quality settings
    let (codec_args, quality_args): (Vec<&str>, Vec<&str>) = match format {
        "mp3" => (vec!["-c:a", "libmp3lame"], vec!["-b:a", "128k"]),
        "ogg" => (vec!["-c:a", "libvorbis"], vec!["-q:a", "4"]),
        "flac" => (vec!["-c:a", "flac"], vec!["-compression_level", "8"]),
        "m4a" | "aac" => (vec!["-c:a", "aac"], vec!["-b:a", "128k"]),
        "opus" => (vec!["-c:a", "libopus"], vec!["-b:a", "96k"]),
        _ => (vec![], vec![]),
    };

    let mut args = vec!["-y", "-i", temp_wav.to_str().unwrap()];
    args.extend(codec_args);
    args.extend(quality_args);
    args.push(output_path);

    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run ffmpeg: {}", e))?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_wav);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg encoding failed: {}", stderr));
    }

    Ok(())
}

// === Command Implementations ===

fn cmd_info(args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err("Missing input file. Usage: info <file>".to_string());
    }

    let input_path = args[0];
    let buffer = load_audio(input_path)?;

    let analyzer = LoudnessAnalyzer::new(buffer.sample_rate);
    let peak_db = analyzer.measure_peak(&buffer.samples);
    let lufs = analyzer.measure_lufs(&buffer.samples);
    let true_peak = analyzer.measure_true_peak(&buffer.samples);
    let dynamic_range = analyzer.measure_dynamic_range(&buffer.samples);

    let mut output = format!("Audio File: {}\n\n", input_path);
    output.push_str(&format!(
        "  Duration:       {:.2}s\n",
        buffer.duration_seconds()
    ));
    output.push_str(&format!("  Sample Rate:    {} Hz\n", buffer.sample_rate));
    output.push_str(&format!("  Channels:       {}\n", buffer.channels));
    output.push_str(&format!("  Frames:         {}\n", buffer.num_frames()));
    output.push_str("\nLoudness Analysis:\n");
    output.push_str(&format!("  Peak:           {:.1} dB\n", peak_db));
    output.push_str(&format!("  True Peak:      {:.1} dB\n", true_peak));
    output.push_str(&format!("  Loudness:       {:.1} LUFS\n", lufs));
    output.push_str(&format!("  Dynamic Range:  {:.1} dB\n", dynamic_range));

    Ok(output)
}

fn cmd_presets() -> Result<String, String> {
    let mut output = String::from("Available Presets:\n\n");

    for preset in Preset::all() {
        output.push_str(&format!(
            "  {:<16} {}\n",
            preset.name(),
            preset.description()
        ));
    }

    output.push_str("\nUsage:\n");
    output.push_str("  adi audio preset -i <input> -o <output> -p <preset>\n");
    output.push_str("  adi audio preset -i <input> -o out.ogg -o out.mp3 -p <preset>\n");
    output.push_str("\nOutput formats: .wav, .mp3, .ogg, .flac, .m4a (requires ffmpeg)\n");
    output.push_str("\nExamples:\n");
    output.push_str("  adi audio preset -i sound.mp3 -o output.ogg -p web-sfx\n");
    output.push_str("  adi audio preset -i sound.mp3 -o out.ogg -o out.mp3 -p web-sfx\n");

    Ok(output)
}

fn cmd_preset(args: &[&str]) -> Result<String, String> {
    if args.is_empty() {
        return Err(
            "Missing arguments. Usage: preset -i <input> -o <output> --preset <name>\n\
             Output formats: .wav, .mp3, .ogg, .flac, .m4a\n\
             Run 'adi audio presets' to see available presets."
                .to_string(),
        );
    }

    // Parse options
    let mut input_path: Option<&str> = None;
    let mut preset_name: Option<&str> = None;
    let mut outputs: Vec<&str> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "-i" | "--input" => {
                if i + 1 < args.len() {
                    input_path = Some(args[i + 1]);
                    i += 2;
                } else {
                    return Err("Missing input path after -i".to_string());
                }
            }
            "--preset" | "-p" => {
                if i + 1 < args.len() {
                    preset_name = Some(args[i + 1]);
                    i += 2;
                } else {
                    return Err("Missing preset name".to_string());
                }
            }
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    outputs.push(args[i + 1]);
                    i += 2;
                } else {
                    return Err("Missing output path after -o".to_string());
                }
            }
            _ => {
                // Support positional args for backward compat: preset <input> <output> <preset>
                if !args[i].starts_with('-') {
                    if input_path.is_none() {
                        input_path = Some(args[i]);
                    } else if outputs.is_empty() && args[i].contains('.') {
                        outputs.push(args[i]);
                    } else if preset_name.is_none() {
                        preset_name = Some(args[i]);
                    }
                }
                i += 1;
            }
        }
    }

    // Validate input
    let input_path = input_path.ok_or_else(|| {
        "Missing input file. Use -i <input> or provide as first argument.".to_string()
    })?;

    // Validate we have at least one output
    if outputs.is_empty() {
        return Err(
            "Missing output file(s). Use -o <output> (can specify multiple: -o a.ogg -o b.mp3)"
                .to_string(),
        );
    }

    let preset_name = preset_name.ok_or_else(|| {
        "Missing preset name. Use --preset <name> or run 'adi audio presets' to see options."
            .to_string()
    })?;

    let preset = Preset::from_str(preset_name).ok_or_else(|| {
        format!(
            "Unknown preset: '{}'\n\nAvailable presets:\n{}",
            preset_name,
            Preset::all()
                .iter()
                .map(|p| format!("  {} - {}", p.name(), p.description()))
                .collect::<Vec<_>>()
                .join("\n")
        )
    })?;

    // Load audio (supports WAV, MP3, etc.)
    let buffer = load_audio(input_path)?;

    // Apply preset (only once)
    let (output_buffer, stats) = apply_preset_with_stats(&buffer, preset);

    // Write to all output files
    let mut output_info: Vec<String> = Vec::new();
    for output_path in &outputs {
        write_audio(output_path, &output_buffer)?;

        let out_ext = Path::new(output_path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_uppercase())
            .unwrap_or_else(|| "WAV".to_string());

        output_info.push(format!("  {} ({})", output_path, out_ext));
    }

    let file_count = outputs.len();
    let plural = if file_count > 1 { "s" } else { "" };

    Ok(format!(
        "Applied preset '{}': {}\n\n\
         Input:\n  Loudness: {:.1} LUFS\n  Peak: {:.1} dB\n\n\
         Output ({} file{}):\n{}\n\n\
         Result:\n  Loudness: {:.1} LUFS\n  Peak: {:.1} dB\n  Gain applied: {:.1} dB",
        stats.preset,
        input_path,
        stats.input_lufs,
        stats.input_peak_db,
        file_count,
        plural,
        output_info.join("\n"),
        stats.output_lufs,
        stats.output_peak_db,
        stats.gain_applied_db
    ))
}

fn cmd_process(args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("Missing arguments. Usage: process <input> <output.wav> [options]".to_string());
    }

    let input_path = args[0];
    let output_path = args[1];

    // Parse options
    let mut highpass_freq: f32 = 80.0;
    let mut compress_preset = "gentle";
    let mut target_lufs: f32 = -14.0;

    let mut i = 2;
    while i < args.len() {
        match args[i] {
            "--highpass" | "-hp" => {
                if i + 1 < args.len() {
                    highpass_freq = args[i + 1]
                        .parse()
                        .map_err(|_| "Invalid highpass frequency")?;
                    i += 2;
                } else {
                    return Err("Missing highpass frequency value".to_string());
                }
            }
            "--compress" | "-c" => {
                if i + 1 < args.len() {
                    compress_preset = args[i + 1];
                    i += 2;
                } else {
                    return Err("Missing compress preset value".to_string());
                }
            }
            "--lufs" | "-l" => {
                if i + 1 < args.len() {
                    target_lufs = args[i + 1].parse().map_err(|_| "Invalid LUFS value")?;
                    i += 2;
                } else {
                    return Err("Missing LUFS value".to_string());
                }
            }
            _ => i += 1,
        }
    }

    // Read input (supports WAV, MP3, etc.)
    let buffer = load_audio(input_path)?;
    let sample_rate = buffer.sample_rate as f32;

    // Process
    let mut output = buffer.clone();

    // 1. High-pass filter
    let mut hpf = HighPassFilter::new(highpass_freq, sample_rate);
    for sample in output.samples.iter_mut() {
        *sample = hpf.process(*sample);
    }

    // 2. EQ (mastering preset)
    let mut eq = ParametricEq::mastering_preset(sample_rate);
    for sample in output.samples.iter_mut() {
        *sample = eq.process(*sample);
    }

    // 3. Compression
    let comp_settings = match compress_preset {
        "vocal" => CompressorSettings::vocal(),
        "aggressive" => CompressorSettings::aggressive(),
        "limiter" => CompressorSettings::limiter(),
        _ => CompressorSettings::gentle(),
    };
    let mut compressor = Compressor::new(comp_settings, sample_rate);
    for sample in output.samples.iter_mut() {
        *sample = compressor.process(*sample);
    }

    // 4. Normalize to target LUFS
    let analyzer = LoudnessAnalyzer::new(buffer.sample_rate);
    let current_lufs = analyzer.measure_lufs(&output.samples);
    let gain_db = target_lufs - current_lufs;
    let normalizer = Normalizer::new(gain_db);
    let mut limiter = Limiter::new(-1.0, sample_rate);

    for sample in output.samples.iter_mut() {
        *sample = normalizer.process(*sample);
        *sample = limiter.process(*sample);
    }

    // Write output
    WavWriter::write(output_path, &output).map_err(|e| format!("Failed to write file: {}", e))?;

    let final_lufs = analyzer.measure_lufs(&output.samples);
    let final_peak = analyzer.measure_peak(&output.samples);

    Ok(format!(
        "Processed: {} -> {}\n\nSettings:\n  High-pass: {} Hz\n  Compression: {}\n  Target LUFS: {:.1}\n\nResult:\n  Loudness: {:.1} LUFS\n  Peak: {:.1} dB",
        input_path, output_path, highpass_freq, compress_preset, target_lufs, final_lufs, final_peak
    ))
}

fn cmd_highpass(args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err(
            "Missing arguments. Usage: highpass <input> <output.wav> --freq <hz>".to_string(),
        );
    }

    let input_path = args[0];
    let output_path = args[1];

    let mut freq: f32 = 80.0;
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--freq" || args[i] == "-f" {
            if i + 1 < args.len() {
                freq = args[i + 1].parse().map_err(|_| "Invalid frequency")?;
            }
        }
        i += 1;
    }

    let buffer = load_audio(input_path)?;
    let mut output = buffer.clone();

    let mut hpf = HighPassFilter::new(freq, buffer.sample_rate as f32);
    for sample in output.samples.iter_mut() {
        *sample = hpf.process(*sample);
    }

    WavWriter::write(output_path, &output).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(format!(
        "Applied {:.0} Hz high-pass filter: {} -> {}",
        freq, input_path, output_path
    ))
}

fn cmd_eq(args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("Missing arguments. Usage: eq <input> <output.wav> [--preset <voice|master>] [--band <type:freq:gain:q>]".to_string());
    }

    let input_path = args[0];
    let output_path = args[1];

    let buffer = load_audio(input_path)?;
    let sample_rate = buffer.sample_rate as f32;
    let mut output = buffer.clone();

    // Check for preset or custom bands
    let mut preset: Option<&str> = None;
    let mut custom_bands: Vec<EqBand> = Vec::new();

    let mut i = 2;
    while i < args.len() {
        match args[i] {
            "--preset" | "-p" => {
                if i + 1 < args.len() {
                    preset = Some(args[i + 1]);
                    i += 2;
                } else {
                    return Err("Missing preset name".to_string());
                }
            }
            "--band" | "-b" => {
                if i + 1 < args.len() {
                    let band = parse_eq_band(args[i + 1])?;
                    custom_bands.push(band);
                    i += 2;
                } else {
                    return Err("Missing band specification".to_string());
                }
            }
            _ => i += 1,
        }
    }

    let mut eq = if let Some(preset_name) = preset {
        match preset_name {
            "voice" => ParametricEq::voice_preset(sample_rate),
            "master" | "mastering" => ParametricEq::mastering_preset(sample_rate),
            _ => {
                return Err(format!(
                    "Unknown preset: {}. Available: voice, master",
                    preset_name
                ))
            }
        }
    } else {
        ParametricEq::new(sample_rate)
    };

    // Add custom bands
    for band in &custom_bands {
        eq.add_band(band.clone());
    }

    if eq.num_bands() == 0 {
        return Err("No EQ bands specified. Use --preset or --band".to_string());
    }

    for sample in output.samples.iter_mut() {
        *sample = eq.process(*sample);
    }

    WavWriter::write(output_path, &output).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(format!(
        "Applied {} EQ bands: {} -> {}",
        eq.num_bands(),
        input_path,
        output_path
    ))
}

fn parse_eq_band(spec: &str) -> Result<EqBand, String> {
    // Format: type:freq:gain:q (e.g., "peak:1000:3:1.5" or "lowshelf:100:2")
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() < 2 {
        return Err("Invalid band format. Use: type:freq[:gain[:q]]".to_string());
    }

    let band_type = match parts[0].to_lowercase().as_str() {
        "peak" | "bell" => EqBandType::Peak,
        "lowshelf" | "ls" => EqBandType::LowShelf,
        "highshelf" | "hs" => EqBandType::HighShelf,
        "highpass" | "hp" => EqBandType::HighPass,
        "lowpass" | "lp" => EqBandType::LowPass,
        "notch" => EqBandType::Notch,
        _ => return Err(format!("Unknown band type: {}", parts[0])),
    };

    let freq: f32 = parts[1].parse().map_err(|_| "Invalid frequency")?;
    let gain: f32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let q: f32 = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.0);

    Ok(EqBand::new(band_type, freq, gain, q))
}

fn cmd_compress(args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("Missing arguments. Usage: compress <input> <output.wav> [--preset <gentle|vocal|aggressive|limiter>]".to_string());
    }

    let input_path = args[0];
    let output_path = args[1];

    let mut preset = "gentle";
    let mut threshold: Option<f32> = None;
    let mut ratio: Option<f32> = None;
    let mut attack: Option<f32> = None;
    let mut release: Option<f32> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i] {
            "--preset" | "-p" => {
                if i + 1 < args.len() {
                    preset = args[i + 1];
                    i += 2;
                } else {
                    return Err("Missing preset name".to_string());
                }
            }
            "--threshold" | "-t" => {
                if i + 1 < args.len() {
                    threshold = Some(args[i + 1].parse().map_err(|_| "Invalid threshold")?);
                    i += 2;
                } else {
                    return Err("Missing threshold value".to_string());
                }
            }
            "--ratio" | "-r" => {
                if i + 1 < args.len() {
                    ratio = Some(args[i + 1].parse().map_err(|_| "Invalid ratio")?);
                    i += 2;
                } else {
                    return Err("Missing ratio value".to_string());
                }
            }
            "--attack" | "-a" => {
                if i + 1 < args.len() {
                    attack = Some(args[i + 1].parse().map_err(|_| "Invalid attack")?);
                    i += 2;
                } else {
                    return Err("Missing attack value".to_string());
                }
            }
            "--release" => {
                if i + 1 < args.len() {
                    release = Some(args[i + 1].parse().map_err(|_| "Invalid release")?);
                    i += 2;
                } else {
                    return Err("Missing release value".to_string());
                }
            }
            _ => i += 1,
        }
    }

    let buffer = load_audio(input_path)?;
    let sample_rate = buffer.sample_rate as f32;
    let mut output = buffer.clone();

    let mut settings = match preset {
        "vocal" => CompressorSettings::vocal(),
        "aggressive" => CompressorSettings::aggressive(),
        "limiter" => CompressorSettings::limiter(),
        _ => CompressorSettings::gentle(),
    };

    // Override with custom values
    if let Some(t) = threshold {
        settings.threshold_db = t;
    }
    if let Some(r) = ratio {
        settings.ratio = r;
    }
    if let Some(a) = attack {
        settings.attack_ms = a;
    }
    if let Some(r) = release {
        settings.release_ms = r;
    }

    let mut compressor = Compressor::new(settings.clone(), sample_rate);
    for sample in output.samples.iter_mut() {
        *sample = compressor.process(*sample);
    }

    WavWriter::write(output_path, &output).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(format!(
        "Applied compression: {} -> {}\n  Threshold: {:.1} dB\n  Ratio: {:.1}:1\n  Attack: {:.1} ms\n  Release: {:.1} ms",
        input_path, output_path, settings.threshold_db, settings.ratio, settings.attack_ms, settings.release_ms
    ))
}

fn cmd_normalize(args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err("Missing arguments. Usage: normalize <input> <output.wav> [--lufs <target>] [--peak <db>]".to_string());
    }

    let input_path = args[0];
    let output_path = args[1];

    let mut target_lufs: Option<f32> = None;
    let mut target_peak: Option<f32> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i] {
            "--lufs" | "-l" => {
                if i + 1 < args.len() {
                    target_lufs = Some(args[i + 1].parse().map_err(|_| "Invalid LUFS value")?);
                    i += 2;
                } else {
                    return Err("Missing LUFS value".to_string());
                }
            }
            "--peak" | "-p" => {
                if i + 1 < args.len() {
                    target_peak = Some(args[i + 1].parse().map_err(|_| "Invalid peak value")?);
                    i += 2;
                } else {
                    return Err("Missing peak value".to_string());
                }
            }
            _ => i += 1,
        }
    }

    if target_lufs.is_none() && target_peak.is_none() {
        target_lufs = Some(-16.0); // Default to -16 LUFS
    }

    let buffer = load_audio(input_path)?;
    let sample_rate = buffer.sample_rate as f32;
    let mut output = buffer.clone();

    let analyzer = LoudnessAnalyzer::new(buffer.sample_rate);

    let normalizer = if let Some(lufs) = target_lufs {
        let current_lufs = analyzer.measure_lufs(&buffer.samples);
        Normalizer::to_lufs(current_lufs, lufs)
    } else if let Some(peak) = target_peak {
        let current_peak = analyzer.measure_peak(&buffer.samples);
        Normalizer::to_peak(current_peak, peak)
    } else {
        Normalizer::new(0.0)
    };

    // Apply normalization with limiting
    let mut limiter = Limiter::new(-0.3, sample_rate); // -0.3 dB ceiling for safety
    for sample in output.samples.iter_mut() {
        *sample = normalizer.process(*sample);
        *sample = limiter.process(*sample);
    }

    WavWriter::write(output_path, &output).map_err(|e| format!("Failed to write file: {}", e))?;

    let final_lufs = analyzer.measure_lufs(&output.samples);
    let final_peak = analyzer.measure_peak(&output.samples);

    Ok(format!(
        "Normalized: {} -> {}\n  Gain applied: {:.1} dB\n  Final loudness: {:.1} LUFS\n  Final peak: {:.1} dB",
        input_path, output_path, normalizer.gain_db(), final_lufs, final_peak
    ))
}

fn cmd_convert(args: &[&str]) -> Result<String, String> {
    if args.len() < 2 {
        return Err(
            "Missing arguments. Usage: convert <input> <output.wav> [--bits <16|24|32>] [--float]"
                .to_string(),
        );
    }

    let input_path = args[0];
    let output_path = args[1];

    let mut bits: u16 = 16;
    let mut use_float = false;

    let mut i = 2;
    while i < args.len() {
        match args[i] {
            "--bits" | "-b" => {
                if i + 1 < args.len() {
                    bits = args[i + 1].parse().map_err(|_| "Invalid bit depth")?;
                    if bits != 16 && bits != 24 && bits != 32 {
                        return Err("Bit depth must be 16, 24, or 32".to_string());
                    }
                    i += 2;
                } else {
                    return Err("Missing bit depth value".to_string());
                }
            }
            "--float" | "-f" => {
                use_float = true;
                i += 1;
            }
            _ => i += 1,
        }
    }

    let buffer = load_audio(input_path)?;

    if use_float {
        WavWriter::write_float(output_path, &buffer)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(format!(
            "Converted: {} -> {} (32-bit float)",
            input_path, output_path
        ))
    } else {
        WavWriter::write_with_bits(output_path, &buffer, bits)
            .map_err(|e| format!("Failed to write file: {}", e))?;
        Ok(format!(
            "Converted: {} -> {} ({}-bit PCM)",
            input_path, output_path, bits
        ))
    }
}
