//! ADI Audio Plugin (v3)
//!
//! Provides CLI commands for audio processing:
//! - info: Analyze audio file
//! - preset: Apply processing preset
//! - process: Apply mastering chain
//! - Various filters and effects

use lib_plugin_abi_v3::*;
use lib_plugin_abi_v3::cli::{CliCommand, CliCommands, CliContext, CliResult};

use adi_audio_core::{
    Compressor, CompressorSettings, EqBand, EqBandType,
    HighPassFilter, LoudnessAnalyzer, Normalizer, ParametricEq, Preset, WavReader,
    WavWriter,
};
use serde_json::json;

pub struct AudioPlugin;

#[async_trait]
impl Plugin for AudioPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "adi.audio".to_string(),
            name: "ADI Audio".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Core,
            author: Some("ADI Team".to_string()),
            description: Some("Audio processing: WAV I/O, filters, EQ, compression, normalization".to_string()),
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl CliCommands for AudioPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            CliCommand {
                name: "info".to_string(),
                description: "Analyze audio file".to_string(),
                usage: "audio info <file>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "preset".to_string(),
                description: "Apply processing preset".to_string(),
                usage: "audio preset <preset-name> <input> <output>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "presets".to_string(),
                description: "List available presets".to_string(),
                usage: "audio presets".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "process".to_string(),
                description: "Apply mastering chain".to_string(),
                usage: "audio process <input> <output>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "highpass".to_string(),
                description: "Apply high-pass filter".to_string(),
                usage: "audio highpass <input> <output> [freq]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "eq".to_string(),
                description: "Apply parametric EQ".to_string(),
                usage: "audio eq <input> <output>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "compress".to_string(),
                description: "Apply dynamic compression".to_string(),
                usage: "audio compress <input> <output>".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "normalize".to_string(),
                description: "Normalize to target loudness".to_string(),
                usage: "audio normalize <input> <output> [target-lufs]".to_string(),
                has_subcommands: false,
            },
            CliCommand {
                name: "convert".to_string(),
                description: "Convert between formats".to_string(),
                usage: "audio convert <input> <output>".to_string(),
                has_subcommands: false,
            },
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        // Get subcommand
        let subcommand = ctx.args.get(0).map(|s| s.as_str()).unwrap_or("");

        match subcommand {
            "info" => {
                if ctx.args.len() < 2 {
                    return Ok(CliResult::error("Usage: audio info <file>"));
                }
                handle_info(&ctx.args[1])
            }
            "preset" => {
                if ctx.args.len() < 4 {
                    return Ok(CliResult::error("Usage: audio preset <preset-name> <input> <output>"));
                }
                handle_preset(&ctx.args[1], &ctx.args[2], &ctx.args[3])
            }
            "presets" => handle_presets(),
            "process" => {
                if ctx.args.len() < 3 {
                    return Ok(CliResult::error("Usage: audio process <input> <output>"));
                }
                handle_process(&ctx.args[1], &ctx.args[2])
            }
            "highpass" => {
                if ctx.args.len() < 3 {
                    return Ok(CliResult::error("Usage: audio highpass <input> <output> [freq]"));
                }
                let freq = ctx.args.get(3).and_then(|s| s.parse().ok()).unwrap_or(80.0);
                handle_highpass(&ctx.args[1], &ctx.args[2], freq)
            }
            "eq" => {
                if ctx.args.len() < 3 {
                    return Ok(CliResult::error("Usage: audio eq <input> <output>"));
                }
                handle_eq(&ctx.args[1], &ctx.args[2])
            }
            "compress" => {
                if ctx.args.len() < 3 {
                    return Ok(CliResult::error("Usage: audio compress <input> <output>"));
                }
                handle_compress(&ctx.args[1], &ctx.args[2])
            }
            "normalize" => {
                if ctx.args.len() < 3 {
                    return Ok(CliResult::error("Usage: audio normalize <input> <output> [target-lufs]"));
                }
                let target = ctx.args.get(3).and_then(|s| s.parse().ok()).unwrap_or(-16.0);
                handle_normalize(&ctx.args[1], &ctx.args[2], target)
            }
            "convert" => {
                if ctx.args.len() < 3 {
                    return Ok(CliResult::error("Usage: audio convert <input> <output>"));
                }
                handle_convert(&ctx.args[1], &ctx.args[2])
            }
            _ => Ok(CliResult::error(&format!("Unknown subcommand: {}", subcommand))),
        }
    }
}

// Command handlers
fn handle_info(file: &str) -> Result<CliResult> {
    let buffer = WavReader::read(file)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to read file: {}", e)))?;

    let analyzer = LoudnessAnalyzer::new(buffer.sample_rate);
    let lufs = analyzer.measure_lufs(&buffer.samples);
    let peak = buffer.samples.iter().fold(0.0f32, |max, &s| max.max(s.abs()));

    let info = json!({
        "channels": buffer.channels,
        "sample_rate": buffer.sample_rate,
        "duration_secs": buffer.duration_seconds(),
        "loudness_lufs": lufs,
        "peak": peak,
    });

    Ok(CliResult::success(serde_json::to_string_pretty(&info).unwrap()))
}

fn handle_preset(preset_name: &str, input: &str, output: &str) -> Result<CliResult> {
    let preset = Preset::from_str(preset_name)
        .ok_or_else(|| PluginError::Other(anyhow::anyhow!("Unknown preset: {}", preset_name)))?;

    let buffer = WavReader::read(input)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to read input: {}", e)))?;

    let (output_buffer, stats) = adi_audio_core::apply_preset_with_stats(&buffer, preset);

    WavWriter::write(output, &output_buffer)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to write output: {}", e)))?;

    Ok(CliResult::success(format!("Processed successfully\n{}", serde_json::to_string_pretty(&stats).unwrap())))
}

fn handle_presets() -> Result<CliResult> {
    let mut output = String::from("Available Presets:\n\n");
    for preset in Preset::all() {
        output.push_str(&format!("  {:<20} {}\n", preset.name(), preset.description()));
    }
    Ok(CliResult::success(output))
}

fn handle_process(input: &str, output: &str) -> Result<CliResult> {
    handle_preset("music-master", input, output)
}

fn handle_highpass(input: &str, output: &str, freq: f32) -> Result<CliResult> {
    let mut buffer = WavReader::read(input)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to read input: {}", e)))?;

    let mut filter = HighPassFilter::new(freq, buffer.sample_rate as f32);

    for sample in buffer.samples.iter_mut() {
        *sample = filter.process(*sample);
    }

    WavWriter::write(output, &buffer)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to write output: {}", e)))?;

    Ok(CliResult::success(format!("Applied high-pass filter ({} Hz)", freq)))
}

fn handle_eq(input: &str, output: &str) -> Result<CliResult> {
    let mut buffer = WavReader::read(input)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to read input: {}", e)))?;

    let bands = vec![
        EqBand::new(EqBandType::LowShelf, 100.0, 2.0, 0.7),
        EqBand::new(EqBandType::Peak, 1000.0, -3.0, 1.0),
        EqBand::new(EqBandType::HighShelf, 8000.0, 1.0, 0.7),
    ];

    let mut eq = ParametricEq::new(buffer.sample_rate as f32);
    for band in bands {
        eq.add_band(band);
    }

    for sample in buffer.samples.iter_mut() {
        *sample = eq.process(*sample);
    }

    WavWriter::write(output, &buffer)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to write output: {}", e)))?;

    Ok(CliResult::success("Applied parametric EQ"))
}

fn handle_compress(input: &str, output: &str) -> Result<CliResult> {
    let mut buffer = WavReader::read(input)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to read input: {}", e)))?;

    let settings = CompressorSettings {
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 5.0,
        release_ms: 100.0,
        knee_db: 6.0,
        makeup_gain_db: 0.0,
    };

    let mut compressor = Compressor::new(settings, buffer.sample_rate as f32);

    for sample in buffer.samples.iter_mut() {
        *sample = compressor.process(*sample);
    }

    WavWriter::write(output, &buffer)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to write output: {}", e)))?;

    Ok(CliResult::success("Applied compression"))
}

fn handle_normalize(input: &str, output: &str, target_lufs: f32) -> Result<CliResult> {
    let mut buffer = WavReader::read(input)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to read input: {}", e)))?;

    // Measure current loudness
    let analyzer = LoudnessAnalyzer::new(buffer.sample_rate);
    let current_lufs = analyzer.measure_lufs(&buffer.samples);
    let gain_db = target_lufs - current_lufs;

    // Apply gain
    let normalizer = Normalizer::new(gain_db);
    for sample in buffer.samples.iter_mut() {
        *sample = normalizer.process(*sample);
    }

    WavWriter::write(output, &buffer)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to write output: {}", e)))?;

    Ok(CliResult::success(format!("Normalized to {} LUFS (gain: {:.2} dB)", target_lufs, gain_db)))
}

fn handle_convert(input: &str, output: &str) -> Result<CliResult> {
    let buffer = WavReader::read(input)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to read input: {}", e)))?;

    WavWriter::write(output, &buffer)
        .map_err(|e| PluginError::Other(anyhow::anyhow!("Failed to write output: {}", e)))?;

    Ok(CliResult::success("Converted successfully"))
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(AudioPlugin)
}
