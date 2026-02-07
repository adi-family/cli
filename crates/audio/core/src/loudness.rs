//! Loudness measurement, normalization, and limiting

/// Loudness analyzer (simplified LUFS-like measurement)
pub struct LoudnessAnalyzer {
    sample_rate: u32,
}

impl LoudnessAnalyzer {
    /// Create a new loudness analyzer
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }

    /// Measure integrated loudness (simplified LUFS approximation)
    ///
    /// This is a simplified RMS-based measurement, not a full ITU-R BS.1770 implementation
    pub fn measure_lufs(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return -70.0;
        }

        // Calculate RMS
        let sum_squared: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
        let rms = (sum_squared / samples.len() as f64).sqrt() as f32;

        if rms < 1e-10 {
            return -70.0;
        }

        // Convert to LUFS-like value
        // Note: Real LUFS requires K-weighting and gating, this is simplified
        -0.691 + 10.0 * rms.log10()
    }

    /// Measure peak level in dB
    pub fn measure_peak(&self, samples: &[f32]) -> f32 {
        let peak = samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        if peak < 1e-10 {
            -96.0
        } else {
            20.0 * peak.log10()
        }
    }

    /// Measure true peak (with oversampling)
    ///
    /// Simple 4x oversampling for true peak detection
    pub fn measure_true_peak(&self, samples: &[f32]) -> f32 {
        let mut max_peak: f32 = 0.0;

        for window in samples.windows(4) {
            if window.len() < 4 {
                continue;
            }

            // Simple cubic interpolation to find inter-sample peaks
            for i in 0..4 {
                let t = i as f32 / 4.0;
                let interpolated = Self::cubic_interpolate(window, t);
                max_peak = max_peak.max(interpolated.abs());
            }
        }

        // Also check original samples
        max_peak = max_peak.max(samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max));

        if max_peak < 1e-10 {
            -96.0
        } else {
            20.0 * max_peak.log10()
        }
    }

    /// Simple cubic interpolation
    fn cubic_interpolate(samples: &[f32], t: f32) -> f32 {
        if samples.len() < 4 {
            return samples.first().copied().unwrap_or(0.0);
        }

        let y0 = samples[0];
        let y1 = samples[1];
        let y2 = samples[2];
        let y3 = samples[3];

        let a0 = y3 - y2 - y0 + y1;
        let a1 = y0 - y1 - a0;
        let a2 = y2 - y0;
        let a3 = y1;

        let t2 = t * t;
        let t3 = t2 * t;

        a0 * t3 + a1 * t2 + a2 * t + a3
    }

    /// Calculate dynamic range (difference between loudest and quietest parts)
    pub fn measure_dynamic_range(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let block_size = (self.sample_rate as usize) / 10; // 100ms blocks
        if block_size == 0 {
            return 0.0;
        }

        let mut block_levels: Vec<f32> = Vec::new();

        for chunk in samples.chunks(block_size) {
            let rms: f32 =
                (chunk.iter().map(|&s| s.powi(2)).sum::<f32>() / chunk.len() as f32).sqrt();
            if rms > 1e-10 {
                block_levels.push(20.0 * rms.log10());
            }
        }

        if block_levels.len() < 2 {
            return 0.0;
        }

        block_levels.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Dynamic range = top 10% average - bottom 10% average
        let bottom_idx = block_levels.len() / 10;
        let top_idx = block_levels.len() - block_levels.len() / 10;

        if bottom_idx >= top_idx {
            return 0.0;
        }

        let bottom_avg: f32 =
            block_levels[..bottom_idx.max(1)].iter().sum::<f32>() / bottom_idx.max(1) as f32;
        let top_avg: f32 = block_levels[top_idx..].iter().sum::<f32>()
            / (block_levels.len() - top_idx).max(1) as f32;

        top_avg - bottom_avg
    }
}

/// Simple gain normalizer
pub struct Normalizer {
    gain_linear: f32,
}

impl Normalizer {
    /// Create a normalizer with specified gain in dB
    pub fn new(gain_db: f32) -> Self {
        Self {
            gain_linear: 10.0_f32.powf(gain_db / 20.0),
        }
    }

    /// Create a normalizer to reach target peak level
    pub fn to_peak(current_peak_db: f32, target_peak_db: f32) -> Self {
        Self::new(target_peak_db - current_peak_db)
    }

    /// Create a normalizer to reach target LUFS
    pub fn to_lufs(current_lufs: f32, target_lufs: f32) -> Self {
        Self::new(target_lufs - current_lufs)
    }

    /// Process a single sample
    pub fn process(&self, input: f32) -> f32 {
        input * self.gain_linear
    }

    /// Get current gain in dB
    pub fn gain_db(&self) -> f32 {
        20.0 * self.gain_linear.log10()
    }
}

/// True peak limiter
pub struct Limiter {
    ceiling_db: f32,
    ceiling_linear: f32,
    release_coeff: f32,
    gain_reduction: f32,
}

impl Limiter {
    /// Create a new limiter
    pub fn new(ceiling_db: f32, sample_rate: f32) -> Self {
        let release_ms = 50.0; // Fixed release time
        let release_coeff = (-1.0 / (release_ms * 0.001 * sample_rate)).exp();

        Self {
            ceiling_db,
            ceiling_linear: 10.0_f32.powf(ceiling_db / 20.0),
            release_coeff,
            gain_reduction: 1.0,
        }
    }

    /// Set ceiling level
    pub fn set_ceiling(&mut self, ceiling_db: f32) {
        self.ceiling_db = ceiling_db;
        self.ceiling_linear = 10.0_f32.powf(ceiling_db / 20.0);
    }

    /// Process a single sample
    pub fn process(&mut self, input: f32) -> f32 {
        let abs_input = input.abs();

        // Calculate required gain reduction
        let target_gain = if abs_input > self.ceiling_linear {
            self.ceiling_linear / abs_input
        } else {
            1.0
        };

        // Smooth gain reduction (instant attack, slow release)
        if target_gain < self.gain_reduction {
            self.gain_reduction = target_gain; // Instant attack
        } else {
            self.gain_reduction =
                self.release_coeff * self.gain_reduction + (1.0 - self.release_coeff) * target_gain;
        }

        input * self.gain_reduction
    }

    /// Get current gain reduction in dB
    pub fn get_gain_reduction_db(&self) -> f32 {
        if self.gain_reduction > 0.0 {
            20.0 * self.gain_reduction.log10()
        } else {
            -96.0
        }
    }

    /// Reset limiter state
    pub fn reset(&mut self) {
        self.gain_reduction = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loudness_analyzer() {
        let analyzer = LoudnessAnalyzer::new(44100);

        // Sine wave at 0 dB peak
        let samples: Vec<f32> = (0..44100)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
            .collect();

        let peak = analyzer.measure_peak(&samples);
        assert!((peak - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_normalizer() {
        let norm = Normalizer::new(6.0); // +6 dB

        let input = 0.5;
        let output = norm.process(input);

        // +6 dB is approximately 2x
        assert!((output - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_limiter() {
        let mut limiter = Limiter::new(-1.0, 44100.0);

        // Process a signal that exceeds ceiling
        let output = limiter.process(2.0);
        assert!(output.abs() <= 10.0_f32.powf(-1.0 / 20.0) + 0.01);
    }
}
