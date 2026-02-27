use thiserror::Error;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, VideoError>;

#[derive(Debug, Error)]
pub enum VideoError {
    #[error("job not found: {0}")]
    JobNotFound(Uuid),

    #[error("invalid phase: expected {expected}, got {actual}")]
    InvalidPhase {
        expected: &'static str,
        actual: String,
    },

    #[error("frame index {index} out of range (total: {total})")]
    FrameOutOfRange { index: u32, total: u32 },

    #[error("encoding failed: {0}")]
    EncodingFailed(String),

    #[error("ffmpeg not found — install FFmpeg to render video")]
    FfmpegNotFound,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
