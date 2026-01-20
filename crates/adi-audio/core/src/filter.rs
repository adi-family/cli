//! Audio filters - High-pass, Low-pass, Biquad

use std::f32::consts::PI;

/// Filter types for biquad filter
#[derive(Debug, Clone, Copy)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
    Peak,
    LowShelf,
    HighShelf,
}

/// Biquad filter coefficients and state
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    // Coefficients
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,

    // State (delay line)
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadFilter {
    /// Create a new biquad filter
    pub fn new(
        filter_type: FilterType,
        frequency: f32,
        sample_rate: f32,
        q: f32,
        gain_db: f32,
    ) -> Self {
        let omega = 2.0 * PI * frequency / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * q);
        let a = 10.0_f32.powf(gain_db / 40.0);

        let (b0, b1, b2, a0, a1, a2) = match filter_type {
            FilterType::LowPass => {
                let b1 = 1.0 - cos_omega;
                let b0 = b1 / 2.0;
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b1 = -(1.0 + cos_omega);
                let b0 = (1.0 + cos_omega) / 2.0;
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::BandPass => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_omega;
                let b2 = 1.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::Peak => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_omega;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let sqrt_a = a.sqrt();
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_omega + 2.0 * sqrt_a * alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_omega);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_omega - 2.0 * sqrt_a * alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_omega + 2.0 * sqrt_a * alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_omega);
                let a2 = (a + 1.0) + (a - 1.0) * cos_omega - 2.0 * sqrt_a * alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let sqrt_a = a.sqrt();
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_omega + 2.0 * sqrt_a * alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_omega);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_omega - 2.0 * sqrt_a * alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_omega + 2.0 * sqrt_a * alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_omega);
                let a2 = (a + 1.0) - (a - 1.0) * cos_omega - 2.0 * sqrt_a * alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        // Normalize coefficients
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    /// Process a single sample
    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;

        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// High-pass filter (convenience wrapper)
pub struct HighPassFilter {
    biquad: BiquadFilter,
}

impl HighPassFilter {
    /// Create a high-pass filter at the given cutoff frequency
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        Self {
            biquad: BiquadFilter::new(FilterType::HighPass, cutoff_hz, sample_rate, 0.707, 0.0),
        }
    }

    /// Create a high-pass filter with custom Q
    pub fn with_q(cutoff_hz: f32, sample_rate: f32, q: f32) -> Self {
        Self {
            biquad: BiquadFilter::new(FilterType::HighPass, cutoff_hz, sample_rate, q, 0.0),
        }
    }

    /// Process a single sample
    pub fn process(&mut self, input: f32) -> f32 {
        self.biquad.process(input)
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.biquad.reset();
    }
}

/// Low-pass filter (convenience wrapper)
pub struct LowPassFilter {
    biquad: BiquadFilter,
}

impl LowPassFilter {
    /// Create a low-pass filter at the given cutoff frequency
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        Self {
            biquad: BiquadFilter::new(FilterType::LowPass, cutoff_hz, sample_rate, 0.707, 0.0),
        }
    }

    /// Process a single sample
    pub fn process(&mut self, input: f32) -> f32 {
        self.biquad.process(input)
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.biquad.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highpass_filter() {
        let mut filter = HighPassFilter::new(100.0, 44100.0);

        // Process some samples
        let output = filter.process(1.0);
        assert!(output.is_finite());
    }

    #[test]
    fn test_biquad_reset() {
        let mut filter = BiquadFilter::new(FilterType::LowPass, 1000.0, 44100.0, 0.707, 0.0);

        // Process some samples
        filter.process(1.0);
        filter.process(0.5);

        // Reset
        filter.reset();

        // State should be zeroed
        assert_eq!(filter.x1, 0.0);
        assert_eq!(filter.y1, 0.0);
    }
}
