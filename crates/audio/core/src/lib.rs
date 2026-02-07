//! ADI Audio Core
//!
//! Audio processing library providing:
//! - WAV file reading/writing
//! - High-pass filter
//! - Parametric EQ
//! - Dynamic range compression
//! - Loudness normalization/limiting
//! - Processing presets (web-sfx, podcast, music-master, etc.)

mod audio;
mod compressor;
mod eq;
mod filter;
mod loudness;
mod preset;

pub use audio::{AudioBuffer, AudioError, AudioFormat, WavReader, WavWriter};
pub use compressor::{Compressor, CompressorSettings};
pub use eq::{EqBand, EqBandType, ParametricEq};
pub use filter::{BiquadFilter, FilterType, HighPassFilter};
pub use loudness::{Limiter, LoudnessAnalyzer, Normalizer};
pub use preset::{apply_preset, apply_preset_with_stats, Preset, PresetResult};

/// Process audio with a full mastering chain
pub fn process_mastering_chain(
    input: &AudioBuffer,
    highpass_freq: f32,
    eq_bands: &[EqBand],
    compressor: &CompressorSettings,
    target_lufs: f32,
) -> Result<AudioBuffer, AudioError> {
    let sample_rate = input.sample_rate;

    // Step 1: High-pass filter
    let mut hpf = HighPassFilter::new(highpass_freq, sample_rate as f32);
    let mut output = input.clone();
    for sample in output.samples.iter_mut() {
        *sample = hpf.process(*sample);
    }

    // Step 2: Parametric EQ
    let mut eq = ParametricEq::new(sample_rate as f32);
    for band in eq_bands {
        eq.add_band(band.clone());
    }
    for sample in output.samples.iter_mut() {
        *sample = eq.process(*sample);
    }

    // Step 3: Compression
    let mut comp = Compressor::new(compressor.clone(), sample_rate as f32);
    for sample in output.samples.iter_mut() {
        *sample = comp.process(*sample);
    }

    // Step 4: Normalize to target loudness with limiting
    let analyzer = LoudnessAnalyzer::new(sample_rate);
    let current_lufs = analyzer.measure_lufs(&output.samples);
    let gain_db = target_lufs - current_lufs;

    let normalizer = Normalizer::new(gain_db);
    let mut limiter = Limiter::new(-1.0, sample_rate as f32); // -1 dB ceiling

    for sample in output.samples.iter_mut() {
        *sample = normalizer.process(*sample);
        *sample = limiter.process(*sample);
    }

    Ok(output)
}
