use std::path::Path;
use std::process::Command;

use crate::{OutputFormat, RenderConfig, Result, VideoError};
use tracing::info;

/// Builds and runs FFmpeg commands per output format.
pub struct FfmpegEncoder;

impl FfmpegEncoder {
    /// Checks that FFmpeg is available on PATH.
    pub fn check_available() -> Result<()> {
        Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map_err(|_| VideoError::FfmpegNotFound)?;
        Ok(())
    }

    /// Encodes numbered JPEG frames into a video file.
    pub fn encode(
        config: &RenderConfig,
        frames_dir: &Path,
        output_path: &Path,
    ) -> Result<()> {
        Self::check_available()?;

        let input_pattern = frames_dir.join("frame_%06d.jpg");

        let status = match config.format {
            OutputFormat::Mp4 => Self::encode_mp4(config, &input_pattern, output_path),
            OutputFormat::Webm => Self::encode_webm(config, &input_pattern, output_path),
            OutputFormat::Gif => Self::encode_gif(config, frames_dir, output_path),
        }?;

        if !status.success() {
            return Err(VideoError::EncodingFailed(format!(
                "ffmpeg exited with code {}",
                status.code().unwrap_or(-1)
            )));
        }

        info!(?output_path, "encoding complete");
        Ok(())
    }

    fn encode_mp4(
        config: &RenderConfig,
        input_pattern: &Path,
        output: &Path,
    ) -> Result<std::process::ExitStatus> {
        Ok(Command::new("ffmpeg")
            .args(["-y", "-framerate"])
            .arg(config.fps.to_string())
            .arg("-i")
            .arg(input_pattern)
            .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf"])
            .arg(config.crf.to_string())
            .arg(output)
            .status()?)
    }

    fn encode_webm(
        config: &RenderConfig,
        input_pattern: &Path,
        output: &Path,
    ) -> Result<std::process::ExitStatus> {
        Ok(Command::new("ffmpeg")
            .args(["-y", "-framerate"])
            .arg(config.fps.to_string())
            .arg("-i")
            .arg(input_pattern)
            .args(["-c:v", "libvpx-vp9", "-crf"])
            .arg(config.crf.to_string())
            .args(["-b:v", "0"])
            .arg(output)
            .status()?)
    }

    fn encode_gif(
        config: &RenderConfig,
        frames_dir: &Path,
        output: &Path,
    ) -> Result<std::process::ExitStatus> {
        let input_pattern = frames_dir.join("frame_%06d.jpg");
        let palette_path = frames_dir.join("palette.png");

        // Pass 1: generate palette
        let palette_status = Command::new("ffmpeg")
            .args(["-y", "-framerate"])
            .arg(config.fps.to_string())
            .arg("-i")
            .arg(&input_pattern)
            .args(["-vf", "palettegen"])
            .arg(&palette_path)
            .status()?;

        if !palette_status.success() {
            return Err(VideoError::EncodingFailed(
                "palette generation failed".into(),
            ));
        }

        // Pass 2: encode with palette
        Ok(Command::new("ffmpeg")
            .args(["-y", "-framerate"])
            .arg(config.fps.to_string())
            .arg("-i")
            .arg(&input_pattern)
            .arg("-i")
            .arg(&palette_path)
            .args(["-lavfi", "paletteuse"])
            .arg(output)
            .status()?)
    }
}
