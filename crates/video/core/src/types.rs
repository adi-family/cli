use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Mp4,
    Webm,
    Gif,
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Webm => "webm",
            Self::Gif => "gif",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub total_frames: u32,
    pub format: OutputFormat,
    #[serde(default = "default_crf")]
    pub crf: u32,
}

fn default_crf() -> u32 {
    23
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RenderPhase {
    Created,
    Capturing,
    Encoding,
    Done,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderJob {
    pub id: Uuid,
    pub config: RenderConfig,
    pub phase: RenderPhase,
    pub frames_received: u32,
    pub progress: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RenderJob {
    pub fn new(config: RenderConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            config,
            phase: RenderPhase::Created,
            frames_received: 0,
            progress: 0.0,
            error: None,
            output_path: None,
            created_at: now,
            updated_at: now,
        }
    }
}
