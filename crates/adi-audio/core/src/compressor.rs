//! Dynamic Range Compression

use serde::{Deserialize, Serialize};

/// Compressor settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressorSettings {
    /// Threshold in dB (compression starts above this level)
    pub threshold_db: f32,
    /// Compression ratio (e.g., 4.0 means 4:1 ratio)
    pub ratio: f32,
    /// Attack time in milliseconds
    pub attack_ms: f32,
    /// Release time in milliseconds
    pub release_ms: f32,
    /// Knee width in dB (0 = hard knee)
    pub knee_db: f32,
    /// Makeup gain in dB
    pub makeup_gain_db: f32,
}

impl Default for CompressorSettings {
    fn default() -> Self {
        Self {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            knee_db: 6.0,
            makeup_gain_db: 0.0,
        }
    }
}

impl CompressorSettings {
    /// Create new compressor settings
    pub fn new(threshold_db: f32, ratio: f32, attack_ms: f32, release_ms: f32) -> Self {
        Self {
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            knee_db: 6.0,
            makeup_gain_db: 0.0,
        }
    }

    /// Set soft knee width
    pub fn with_knee(mut self, knee_db: f32) -> Self {
        self.knee_db = knee_db;
        self
    }

    /// Set makeup gain
    pub fn with_makeup_gain(mut self, gain_db: f32) -> Self {
        self.makeup_gain_db = gain_db;
        self
    }

    /// Gentle compression preset (for mastering)
    pub fn gentle() -> Self {
        Self {
            threshold_db: -24.0,
            ratio: 2.0,
            attack_ms: 30.0,
            release_ms: 200.0,
            knee_db: 12.0,
            makeup_gain_db: 3.0,
        }
    }

    /// Vocal compression preset
    pub fn vocal() -> Self {
        Self {
            threshold_db: -18.0,
            ratio: 3.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            knee_db: 6.0,
            makeup_gain_db: 4.0,
        }
    }

    /// Aggressive compression preset
    pub fn aggressive() -> Self {
        Self {
            threshold_db: -30.0,
            ratio: 8.0,
            attack_ms: 1.0,
            release_ms: 50.0,
            knee_db: 0.0,
            makeup_gain_db: 10.0,
        }
    }

    /// Limiting preset (brick wall)
    pub fn limiter() -> Self {
        Self {
            threshold_db: -1.0,
            ratio: 20.0,
            attack_ms: 0.1,
            release_ms: 50.0,
            knee_db: 0.0,
            makeup_gain_db: 0.0,
        }
    }
}

/// Dynamic range compressor
pub struct Compressor {
    settings: CompressorSettings,
    sample_rate: f32,

    // Envelope follower state
    envelope_db: f32,
    attack_coeff: f32,
    release_coeff: f32,
}

impl Compressor {
    /// Create a new compressor
    pub fn new(settings: CompressorSettings, sample_rate: f32) -> Self {
        let attack_coeff = Self::time_to_coeff(settings.attack_ms, sample_rate);
        let release_coeff = Self::time_to_coeff(settings.release_ms, sample_rate);

        Self {
            settings,
            sample_rate,
            envelope_db: -96.0,
            attack_coeff,
            release_coeff,
        }
    }

    /// Convert time in ms to coefficient
    fn time_to_coeff(time_ms: f32, sample_rate: f32) -> f32 {
        if time_ms <= 0.0 {
            return 0.0;
        }
        (-1.0 / (time_ms * 0.001 * sample_rate)).exp()
    }

    /// Convert linear amplitude to dB
    fn linear_to_db(linear: f32) -> f32 {
        if linear.abs() < 1e-10 {
            -96.0
        } else {
            20.0 * linear.abs().log10()
        }
    }

    /// Convert dB to linear amplitude
    fn db_to_linear(db: f32) -> f32 {
        10.0_f32.powf(db / 20.0)
    }

    /// Calculate gain reduction using soft knee
    fn compute_gain_reduction(&self, input_db: f32) -> f32 {
        let threshold = self.settings.threshold_db;
        let ratio = self.settings.ratio;
        let knee = self.settings.knee_db;

        if knee <= 0.0 {
            // Hard knee
            if input_db <= threshold {
                0.0
            } else {
                (threshold - input_db) * (1.0 - 1.0 / ratio)
            }
        } else {
            // Soft knee
            let knee_start = threshold - knee / 2.0;
            let knee_end = threshold + knee / 2.0;

            if input_db <= knee_start {
                0.0
            } else if input_db >= knee_end {
                (threshold - input_db) * (1.0 - 1.0 / ratio)
            } else {
                // In the knee region - quadratic interpolation
                let x = input_db - knee_start;
                let knee_curve = (1.0 - 1.0 / ratio) / (2.0 * knee);
                -knee_curve * x * x
            }
        }
    }

    /// Process a single sample
    pub fn process(&mut self, input: f32) -> f32 {
        // Convert to dB
        let input_db = Self::linear_to_db(input);

        // Envelope follower (peak detection)
        let coeff = if input_db > self.envelope_db {
            self.attack_coeff
        } else {
            self.release_coeff
        };
        self.envelope_db = coeff * self.envelope_db + (1.0 - coeff) * input_db;

        // Compute gain reduction
        let gain_reduction_db = self.compute_gain_reduction(self.envelope_db);

        // Apply gain reduction and makeup gain
        let output_gain_db = gain_reduction_db + self.settings.makeup_gain_db;
        let output_gain = Self::db_to_linear(output_gain_db);

        input * output_gain
    }

    /// Reset compressor state
    pub fn reset(&mut self) {
        self.envelope_db = -96.0;
    }

    /// Get current gain reduction in dB
    pub fn get_gain_reduction(&self) -> f32 {
        -self.compute_gain_reduction(self.envelope_db)
    }

    /// Update settings
    pub fn set_settings(&mut self, settings: CompressorSettings) {
        self.attack_coeff = Self::time_to_coeff(settings.attack_ms, self.sample_rate);
        self.release_coeff = Self::time_to_coeff(settings.release_ms, self.sample_rate);
        self.settings = settings;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressor_basic() {
        let settings = CompressorSettings::default();
        let mut comp = Compressor::new(settings, 44100.0);

        // Process a loud signal
        let output = comp.process(1.0);
        assert!(output.is_finite());
        assert!(output.abs() <= 1.0);
    }

    #[test]
    fn test_compressor_quiet_signal() {
        let settings = CompressorSettings::default();
        let mut comp = Compressor::new(settings, 44100.0);

        // Quiet signal should pass through mostly unchanged
        let input = 0.01; // About -40 dB
        let output = comp.process(input);
        assert!((output - input * 1.0).abs() < 0.1);
    }

    #[test]
    fn test_compressor_presets() {
        let gentle = CompressorSettings::gentle();
        assert!(gentle.ratio < 3.0);

        let aggressive = CompressorSettings::aggressive();
        assert!(aggressive.ratio > 5.0);
    }
}
