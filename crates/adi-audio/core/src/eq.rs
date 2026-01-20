//! Parametric Equalizer

use crate::filter::{BiquadFilter, FilterType};
use serde::{Deserialize, Serialize};

/// EQ band type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EqBandType {
    /// Low shelf filter
    LowShelf,
    /// High shelf filter
    HighShelf,
    /// Peaking/bell filter
    Peak,
    /// High-pass filter
    HighPass,
    /// Low-pass filter
    LowPass,
    /// Notch filter
    Notch,
}

impl From<EqBandType> for FilterType {
    fn from(band_type: EqBandType) -> Self {
        match band_type {
            EqBandType::LowShelf => FilterType::LowShelf,
            EqBandType::HighShelf => FilterType::HighShelf,
            EqBandType::Peak => FilterType::Peak,
            EqBandType::HighPass => FilterType::HighPass,
            EqBandType::LowPass => FilterType::LowPass,
            EqBandType::Notch => FilterType::Notch,
        }
    }
}

/// A single EQ band configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBand {
    /// Band type
    pub band_type: EqBandType,
    /// Center/cutoff frequency in Hz
    pub frequency: f32,
    /// Gain in dB (for shelf and peak filters)
    pub gain_db: f32,
    /// Q factor (bandwidth)
    pub q: f32,
    /// Whether this band is enabled
    pub enabled: bool,
}

impl EqBand {
    /// Create a new EQ band
    pub fn new(band_type: EqBandType, frequency: f32, gain_db: f32, q: f32) -> Self {
        Self {
            band_type,
            frequency,
            gain_db,
            q,
            enabled: true,
        }
    }

    /// Create a low shelf band
    pub fn low_shelf(frequency: f32, gain_db: f32) -> Self {
        Self::new(EqBandType::LowShelf, frequency, gain_db, 0.707)
    }

    /// Create a high shelf band
    pub fn high_shelf(frequency: f32, gain_db: f32) -> Self {
        Self::new(EqBandType::HighShelf, frequency, gain_db, 0.707)
    }

    /// Create a peak/bell band
    pub fn peak(frequency: f32, gain_db: f32, q: f32) -> Self {
        Self::new(EqBandType::Peak, frequency, gain_db, q)
    }

    /// Create a high-pass band
    pub fn high_pass(frequency: f32, q: f32) -> Self {
        Self::new(EqBandType::HighPass, frequency, 0.0, q)
    }

    /// Create a low-pass band
    pub fn low_pass(frequency: f32, q: f32) -> Self {
        Self::new(EqBandType::LowPass, frequency, 0.0, q)
    }
}

/// Parametric equalizer with multiple bands
pub struct ParametricEq {
    bands: Vec<(EqBand, BiquadFilter)>,
    sample_rate: f32,
}

impl ParametricEq {
    /// Create a new parametric EQ
    pub fn new(sample_rate: f32) -> Self {
        Self {
            bands: Vec::new(),
            sample_rate,
        }
    }

    /// Add an EQ band
    pub fn add_band(&mut self, band: EqBand) {
        let filter = BiquadFilter::new(
            band.band_type.into(),
            band.frequency,
            self.sample_rate,
            band.q,
            band.gain_db,
        );
        self.bands.push((band, filter));
    }

    /// Remove all bands
    pub fn clear(&mut self) {
        self.bands.clear();
    }

    /// Get number of bands
    pub fn num_bands(&self) -> usize {
        self.bands.len()
    }

    /// Process a single sample through all bands
    pub fn process(&mut self, input: f32) -> f32 {
        let mut output = input;

        for (band, filter) in &mut self.bands {
            if band.enabled {
                output = filter.process(output);
            }
        }

        output
    }

    /// Reset all filter states
    pub fn reset(&mut self) {
        for (_, filter) in &mut self.bands {
            filter.reset();
        }
    }

    /// Create a basic voice EQ preset
    pub fn voice_preset(sample_rate: f32) -> Self {
        let mut eq = Self::new(sample_rate);
        // High-pass to remove rumble
        eq.add_band(EqBand::high_pass(80.0, 0.707));
        // Reduce mud
        eq.add_band(EqBand::peak(250.0, -3.0, 1.0));
        // Add presence
        eq.add_band(EqBand::peak(3000.0, 2.0, 1.5));
        // Add air
        eq.add_band(EqBand::high_shelf(10000.0, 2.0));
        eq
    }

    /// Create a basic music mastering EQ preset
    pub fn mastering_preset(sample_rate: f32) -> Self {
        let mut eq = Self::new(sample_rate);
        // Subsonic filter
        eq.add_band(EqBand::high_pass(30.0, 0.707));
        // Bass boost
        eq.add_band(EqBand::low_shelf(100.0, 1.5));
        // High shelf air
        eq.add_band(EqBand::high_shelf(12000.0, 1.0));
        eq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parametric_eq_basic() {
        let mut eq = ParametricEq::new(44100.0);
        eq.add_band(EqBand::peak(1000.0, 3.0, 1.0));

        let output = eq.process(1.0);
        assert!(output.is_finite());
    }

    #[test]
    fn test_eq_preset() {
        let mut eq = ParametricEq::voice_preset(44100.0);
        assert_eq!(eq.num_bands(), 4);

        let output = eq.process(0.5);
        assert!(output.is_finite());
    }
}
