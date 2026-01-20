//! Audio buffer and WAV I/O

use hound::{SampleFormat, WavSpec};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WAV error: {0}")]
    Wav(#[from] hound::Error),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

/// Audio format specification
#[derive(Debug, Clone, Copy)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
            bits_per_sample: 16,
        }
    }
}

/// Audio buffer holding samples in f32 format (normalized -1.0 to 1.0)
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    /// Interleaved samples (for stereo: L, R, L, R, ...)
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioBuffer {
    /// Create an empty audio buffer
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            samples: Vec::new(),
            sample_rate,
            channels,
        }
    }

    /// Create buffer with pre-allocated capacity
    pub fn with_capacity(sample_rate: u32, channels: u16, num_frames: usize) -> Self {
        Self {
            samples: Vec::with_capacity(num_frames * channels as usize),
            sample_rate,
            channels,
        }
    }

    /// Number of audio frames (samples / channels)
    pub fn num_frames(&self) -> usize {
        self.samples.len() / self.channels as usize
    }

    /// Duration in seconds
    pub fn duration_seconds(&self) -> f32 {
        self.num_frames() as f32 / self.sample_rate as f32
    }

    /// Get a specific channel as a separate buffer (mono)
    pub fn get_channel(&self, channel: u16) -> Vec<f32> {
        if channel >= self.channels {
            return Vec::new();
        }
        self.samples
            .iter()
            .skip(channel as usize)
            .step_by(self.channels as usize)
            .copied()
            .collect()
    }

    /// Convert stereo to mono by averaging channels
    pub fn to_mono(&self) -> AudioBuffer {
        if self.channels == 1 {
            return self.clone();
        }

        let mono_samples: Vec<f32> = self
            .samples
            .chunks(self.channels as usize)
            .map(|frame| frame.iter().sum::<f32>() / self.channels as f32)
            .collect();

        AudioBuffer {
            samples: mono_samples,
            sample_rate: self.sample_rate,
            channels: 1,
        }
    }
}

/// WAV file reader
pub struct WavReader;

impl WavReader {
    /// Read a WAV file into an AudioBuffer
    pub fn read<P: AsRef<Path>>(path: P) -> Result<AudioBuffer, AudioError> {
        let reader = hound::WavReader::open(path)?;
        let spec = reader.spec();

        let samples: Vec<f32> = match spec.sample_format {
            SampleFormat::Int => {
                let max_value = (1i64 << (spec.bits_per_sample - 1)) as f32;
                match spec.bits_per_sample {
                    8 => reader
                        .into_samples::<i8>()
                        .map(|s| s.map(|v| v as f32 / max_value))
                        .collect::<Result<Vec<_>, _>>()?,
                    16 => reader
                        .into_samples::<i16>()
                        .map(|s| s.map(|v| v as f32 / max_value))
                        .collect::<Result<Vec<_>, _>>()?,
                    24 | 32 => reader
                        .into_samples::<i32>()
                        .map(|s| s.map(|v| v as f32 / max_value))
                        .collect::<Result<Vec<_>, _>>()?,
                    _ => {
                        return Err(AudioError::UnsupportedFormat(format!(
                            "Unsupported bit depth: {}",
                            spec.bits_per_sample
                        )))
                    }
                }
            }
            SampleFormat::Float => reader
                .into_samples::<f32>()
                .collect::<Result<Vec<_>, _>>()?,
        };

        Ok(AudioBuffer {
            samples,
            sample_rate: spec.sample_rate,
            channels: spec.channels,
        })
    }
}

/// WAV file writer
pub struct WavWriter;

impl WavWriter {
    /// Write an AudioBuffer to a WAV file (16-bit PCM)
    pub fn write<P: AsRef<Path>>(path: P, buffer: &AudioBuffer) -> Result<(), AudioError> {
        Self::write_with_bits(path, buffer, 16)
    }

    /// Write an AudioBuffer to a WAV file with specified bit depth
    pub fn write_with_bits<P: AsRef<Path>>(
        path: P,
        buffer: &AudioBuffer,
        bits: u16,
    ) -> Result<(), AudioError> {
        let spec = WavSpec {
            channels: buffer.channels,
            sample_rate: buffer.sample_rate,
            bits_per_sample: bits,
            sample_format: SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(path, spec)?;
        let max_value = (1i64 << (bits - 1)) as f32;

        for &sample in &buffer.samples {
            // Clamp to prevent clipping
            let clamped = sample.clamp(-1.0, 1.0);
            let int_sample = (clamped * max_value) as i32;

            match bits {
                8 => writer.write_sample(int_sample as i8)?,
                16 => writer.write_sample(int_sample as i16)?,
                24 | 32 => writer.write_sample(int_sample)?,
                _ => {
                    return Err(AudioError::UnsupportedFormat(format!(
                        "Unsupported bit depth: {}",
                        bits
                    )))
                }
            }
        }

        writer.finalize()?;
        Ok(())
    }

    /// Write an AudioBuffer as 32-bit float WAV
    pub fn write_float<P: AsRef<Path>>(path: P, buffer: &AudioBuffer) -> Result<(), AudioError> {
        let spec = WavSpec {
            channels: buffer.channels,
            sample_rate: buffer.sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };

        let mut writer = hound::WavWriter::create(path, spec)?;

        for &sample in &buffer.samples {
            writer.write_sample(sample)?;
        }

        writer.finalize()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_buffer_mono_conversion() {
        let buffer = AudioBuffer {
            samples: vec![0.5, 0.3, -0.5, -0.3, 0.2, 0.1],
            sample_rate: 44100,
            channels: 2,
        };

        let mono = buffer.to_mono();
        assert_eq!(mono.channels, 1);
        assert_eq!(mono.samples.len(), 3);
        assert!((mono.samples[0] - 0.4).abs() < 0.001); // (0.5 + 0.3) / 2
    }

    #[test]
    fn test_audio_buffer_duration() {
        let buffer = AudioBuffer {
            samples: vec![0.0; 88200], // 2 seconds at 44100 Hz stereo
            sample_rate: 44100,
            channels: 2,
        };

        assert!((buffer.duration_seconds() - 1.0).abs() < 0.001);
    }
}
