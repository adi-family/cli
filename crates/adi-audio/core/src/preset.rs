//! Audio processing presets for common use cases

use crate::{
    AudioBuffer, Compressor, CompressorSettings, EqBand, HighPassFilter, Limiter, LoudnessAnalyzer,
    Normalizer, ParametricEq,
};
use serde::{Deserialize, Serialize};

/// Available audio processing presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    /// Web sound effect: punchy, loud, clean
    /// - High-pass at 60Hz (remove rumble)
    /// - Presence boost at 2-4kHz
    /// - Fast compression for punch
    /// - Normalize to -10 LUFS (loud for UI)
    /// - Hard limit at -0.5 dB
    WebSfx,

    /// Web notification: clear, not too loud
    /// - High-pass at 100Hz
    /// - Gentle presence boost
    /// - Light compression
    /// - Normalize to -16 LUFS
    WebNotification,

    /// Podcast/voice: clear speech
    /// - High-pass at 80Hz
    /// - Reduce mud (200-400Hz)
    /// - Boost presence (2-5kHz)
    /// - Voice compression
    /// - Normalize to -16 LUFS (podcast standard)
    Podcast,

    /// Music mastering: balanced, dynamic
    /// - Gentle high-pass at 30Hz
    /// - Subtle low shelf boost
    /// - Air boost at 12kHz
    /// - Gentle compression
    /// - Normalize to -14 LUFS (streaming standard)
    MusicMaster,

    /// Voice-over: broadcast quality
    /// - High-pass at 80Hz
    /// - De-mud at 300Hz
    /// - Presence at 3kHz
    /// - Broadcast compression
    /// - Normalize to -24 LUFS (broadcast standard)
    Broadcast,

    /// Ringtone: very loud and punchy
    /// - High-pass at 80Hz
    /// - Strong mid boost
    /// - Heavy compression
    /// - Normalize to -8 LUFS
    Ringtone,

    /// Game SFX: punchy with headroom
    /// - High-pass at 40Hz
    /// - Subtle EQ
    /// - Medium compression
    /// - Normalize to -12 LUFS
    GameSfx,
}

impl Preset {
    /// Get all available presets
    pub fn all() -> &'static [Preset] {
        &[
            Preset::WebSfx,
            Preset::WebNotification,
            Preset::Podcast,
            Preset::MusicMaster,
            Preset::Broadcast,
            Preset::Ringtone,
            Preset::GameSfx,
        ]
    }

    /// Get preset name
    pub fn name(&self) -> &'static str {
        match self {
            Preset::WebSfx => "web-sfx",
            Preset::WebNotification => "web-notification",
            Preset::Podcast => "podcast",
            Preset::MusicMaster => "music-master",
            Preset::Broadcast => "broadcast",
            Preset::Ringtone => "ringtone",
            Preset::GameSfx => "game-sfx",
        }
    }

    /// Get preset description
    pub fn description(&self) -> &'static str {
        match self {
            Preset::WebSfx => "Web UI sound effect: punchy, loud, clean (-10 LUFS)",
            Preset::WebNotification => "Web notification: clear, moderate volume (-16 LUFS)",
            Preset::Podcast => "Podcast/voice: clear speech, podcast standard (-16 LUFS)",
            Preset::MusicMaster => "Music mastering: balanced, streaming standard (-14 LUFS)",
            Preset::Broadcast => "Broadcast voice-over: broadcast standard (-24 LUFS)",
            Preset::Ringtone => "Ringtone: very loud and attention-grabbing (-8 LUFS)",
            Preset::GameSfx => "Game sound effect: punchy with headroom (-12 LUFS)",
        }
    }

    /// Parse preset from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "web-sfx" | "websfx" | "web" | "sfx" => Some(Preset::WebSfx),
            "web-notification" | "notification" | "notif" => Some(Preset::WebNotification),
            "podcast" | "voice" => Some(Preset::Podcast),
            "music-master" | "music" | "master" | "mastering" => Some(Preset::MusicMaster),
            "broadcast" | "radio" | "tv" => Some(Preset::Broadcast),
            "ringtone" | "ring" | "alarm" => Some(Preset::Ringtone),
            "game-sfx" | "game" => Some(Preset::GameSfx),
            _ => None,
        }
    }

    /// Get high-pass filter frequency
    fn highpass_freq(&self) -> f32 {
        match self {
            Preset::WebSfx => 60.0,
            Preset::WebNotification => 100.0,
            Preset::Podcast => 80.0,
            Preset::MusicMaster => 30.0,
            Preset::Broadcast => 80.0,
            Preset::Ringtone => 80.0,
            Preset::GameSfx => 40.0,
        }
    }

    /// Get target loudness in LUFS
    fn target_lufs(&self) -> f32 {
        match self {
            Preset::WebSfx => -10.0,
            Preset::WebNotification => -16.0,
            Preset::Podcast => -16.0,
            Preset::MusicMaster => -14.0,
            Preset::Broadcast => -24.0,
            Preset::Ringtone => -8.0,
            Preset::GameSfx => -12.0,
        }
    }

    /// Get limiter ceiling in dB
    fn limiter_ceiling(&self) -> f32 {
        match self {
            Preset::WebSfx => -0.5,
            Preset::WebNotification => -1.0,
            Preset::Podcast => -1.0,
            Preset::MusicMaster => -1.0,
            Preset::Broadcast => -2.0,
            Preset::Ringtone => -0.3,
            Preset::GameSfx => -1.0,
        }
    }

    /// Get compressor settings
    fn compressor_settings(&self) -> CompressorSettings {
        match self {
            Preset::WebSfx => CompressorSettings {
                threshold_db: -18.0,
                ratio: 4.0,
                attack_ms: 5.0,
                release_ms: 50.0,
                knee_db: 6.0,
                makeup_gain_db: 2.0,
            },
            Preset::WebNotification => CompressorSettings {
                threshold_db: -20.0,
                ratio: 2.5,
                attack_ms: 10.0,
                release_ms: 100.0,
                knee_db: 8.0,
                makeup_gain_db: 1.0,
            },
            Preset::Podcast => CompressorSettings::vocal(),
            Preset::MusicMaster => CompressorSettings::gentle(),
            Preset::Broadcast => CompressorSettings {
                threshold_db: -20.0,
                ratio: 3.5,
                attack_ms: 8.0,
                release_ms: 80.0,
                knee_db: 6.0,
                makeup_gain_db: 3.0,
            },
            Preset::Ringtone => CompressorSettings {
                threshold_db: -15.0,
                ratio: 6.0,
                attack_ms: 2.0,
                release_ms: 30.0,
                knee_db: 3.0,
                makeup_gain_db: 4.0,
            },
            Preset::GameSfx => CompressorSettings {
                threshold_db: -20.0,
                ratio: 3.0,
                attack_ms: 5.0,
                release_ms: 60.0,
                knee_db: 6.0,
                makeup_gain_db: 2.0,
            },
        }
    }

    /// Get EQ bands for this preset
    fn eq_bands(&self) -> Vec<EqBand> {
        match self {
            Preset::WebSfx => vec![
                EqBand::peak(2500.0, 2.0, 1.5),  // Presence boost
                EqBand::peak(4000.0, 1.5, 1.2),  // Clarity
                EqBand::high_shelf(8000.0, 1.0), // Air
            ],
            Preset::WebNotification => vec![
                EqBand::peak(3000.0, 1.5, 1.5), // Gentle presence
                EqBand::high_shelf(6000.0, 1.0),
            ],
            Preset::Podcast => vec![
                EqBand::peak(300.0, -2.0, 1.0),   // Reduce mud
                EqBand::peak(3000.0, 2.5, 1.5),   // Presence
                EqBand::high_shelf(10000.0, 1.5), // Air
            ],
            Preset::MusicMaster => vec![
                EqBand::low_shelf(100.0, 1.0),    // Subtle bass warmth
                EqBand::high_shelf(12000.0, 1.0), // Air
            ],
            Preset::Broadcast => vec![
                EqBand::peak(300.0, -3.0, 1.0),  // De-mud
                EqBand::peak(3000.0, 2.0, 1.5),  // Presence
                EqBand::peak(5000.0, 1.0, 1.5),  // Clarity
                EqBand::high_shelf(8000.0, 0.5), // Subtle air
            ],
            Preset::Ringtone => vec![
                EqBand::peak(1000.0, 2.0, 1.0), // Mid boost for phone speakers
                EqBand::peak(3000.0, 3.0, 1.5), // Strong presence
            ],
            Preset::GameSfx => vec![
                EqBand::low_shelf(80.0, 1.0),   // Bass punch
                EqBand::peak(2500.0, 1.5, 1.5), // Presence
            ],
        }
    }
}

/// Apply a preset to an audio buffer
pub fn apply_preset(input: &AudioBuffer, preset: Preset) -> AudioBuffer {
    let sample_rate = input.sample_rate as f32;
    let mut output = input.clone();

    // Step 1: High-pass filter
    let mut hpf = HighPassFilter::new(preset.highpass_freq(), sample_rate);
    for sample in output.samples.iter_mut() {
        *sample = hpf.process(*sample);
    }

    // Step 2: EQ
    let mut eq = ParametricEq::new(sample_rate);
    for band in preset.eq_bands() {
        eq.add_band(band);
    }
    for sample in output.samples.iter_mut() {
        *sample = eq.process(*sample);
    }

    // Step 3: Compression
    let mut compressor = Compressor::new(preset.compressor_settings(), sample_rate);
    for sample in output.samples.iter_mut() {
        *sample = compressor.process(*sample);
    }

    // Step 4: Normalize to target loudness
    let analyzer = LoudnessAnalyzer::new(input.sample_rate);
    let current_lufs = analyzer.measure_lufs(&output.samples);
    let gain_db = preset.target_lufs() - current_lufs;

    let normalizer = Normalizer::new(gain_db);
    let mut limiter = Limiter::new(preset.limiter_ceiling(), sample_rate);

    for sample in output.samples.iter_mut() {
        *sample = normalizer.process(*sample);
        *sample = limiter.process(*sample);
    }

    output
}

/// Preset processing result with stats
#[derive(Debug, Clone, Serialize)]
pub struct PresetResult {
    pub preset: String,
    pub input_lufs: f32,
    pub output_lufs: f32,
    pub input_peak_db: f32,
    pub output_peak_db: f32,
    pub gain_applied_db: f32,
}

/// Apply preset and return detailed results
pub fn apply_preset_with_stats(input: &AudioBuffer, preset: Preset) -> (AudioBuffer, PresetResult) {
    let analyzer = LoudnessAnalyzer::new(input.sample_rate);
    let input_lufs = analyzer.measure_lufs(&input.samples);
    let input_peak = analyzer.measure_peak(&input.samples);

    let output = apply_preset(input, preset);

    let output_lufs = analyzer.measure_lufs(&output.samples);
    let output_peak = analyzer.measure_peak(&output.samples);

    let stats = PresetResult {
        preset: preset.name().to_string(),
        input_lufs,
        output_lufs,
        input_peak_db: input_peak,
        output_peak_db: output_peak,
        gain_applied_db: output_lufs - input_lufs,
    };

    (output, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_parsing() {
        assert_eq!(Preset::from_str("web-sfx"), Some(Preset::WebSfx));
        assert_eq!(Preset::from_str("sfx"), Some(Preset::WebSfx));
        assert_eq!(Preset::from_str("podcast"), Some(Preset::Podcast));
        assert_eq!(Preset::from_str("unknown"), None);
    }

    #[test]
    fn test_all_presets() {
        assert_eq!(Preset::all().len(), 7);
    }

    #[test]
    fn test_apply_preset() {
        let buffer = AudioBuffer {
            samples: vec![0.5, -0.5, 0.3, -0.3],
            sample_rate: 44100,
            channels: 1,
        };

        let output = apply_preset(&buffer, Preset::WebSfx);
        assert_eq!(output.samples.len(), buffer.samples.len());
    }
}
