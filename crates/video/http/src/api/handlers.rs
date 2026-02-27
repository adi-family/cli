use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use video_core::{FfmpegEncoder, RenderConfig, RenderPhase};

use crate::AppState;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

fn err(status: StatusCode, msg: impl Into<String>) -> impl IntoResponse {
    (status, Json(ErrorResponse { error: msg.into() }))
}

pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "healthy" }))
}

#[derive(Deserialize)]
pub struct CreateRenderRequest {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub total_frames: u32,
    pub format: video_core::OutputFormat,
    pub crf: Option<u32>,
}

#[derive(Serialize)]
pub struct CreateRenderResponse {
    pub job_id: Uuid,
}

pub async fn create_render(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateRenderRequest>,
) -> impl IntoResponse {
    let config = RenderConfig {
        width: req.width,
        height: req.height,
        fps: req.fps,
        total_frames: req.total_frames,
        format: req.format,
        crf: req.crf.unwrap_or(23),
    };

    let job = state.jobs.create(config);
    if let Err(e) = state.frames.ensure_job_dir(job.id) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e.to_string() })),
        ).into_response();
    }

    (StatusCode::CREATED, Json(CreateRenderResponse { job_id: job.id })).into_response()
}

#[derive(Deserialize)]
pub struct FrameQuery {
    pub index: u32,
}

pub async fn upload_frame(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(query): Query<FrameQuery>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let job = match state.jobs.get(id) {
        Ok(j) => j,
        Err(_) => return err(StatusCode::NOT_FOUND, "job not found").into_response(),
    };

    if query.index >= job.config.total_frames {
        return err(StatusCode::BAD_REQUEST, format!(
            "frame index {} out of range (total: {})",
            query.index, job.config.total_frames,
        )).into_response();
    }

    if let Err(e) = state.frames.save_frame(id, query.index, &body) {
        return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    if let Err(e) = state.jobs.increment_frames(id) {
        return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    (StatusCode::OK, Json(serde_json::json!({ "ok": true }))).into_response()
}

pub async fn finish_upload(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let job = match state.jobs.get(id) {
        Ok(j) => j,
        Err(_) => return err(StatusCode::NOT_FOUND, "job not found").into_response(),
    };

    let _ = state.jobs.update(id, |j| {
        j.phase = RenderPhase::Encoding;
        j.progress = 0.0;
    });

    let state_clone = state.clone();
    let config = job.config.clone();
    tokio::spawn(async move {
        let frames_dir = state_clone.frames.job_dir(id);
        let output_path = state_clone.frames.output_path(id, config.format.extension());

        match FfmpegEncoder::encode(&config, &frames_dir, &output_path) {
            Ok(()) => {
                let _ = state_clone.jobs.update(id, |j| {
                    j.phase = RenderPhase::Done;
                    j.progress = 1.0;
                    j.output_path = Some(output_path.to_string_lossy().to_string());
                });
            }
            Err(e) => {
                let _ = state_clone.jobs.update(id, |j| {
                    j.phase = RenderPhase::Error;
                    j.error = Some(e.to_string());
                });
            }
        }
    });

    (StatusCode::ACCEPTED, Json(serde_json::json!({ "encoding": true }))).into_response()
}

pub async fn get_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.jobs.get(id) {
        Ok(job) => (StatusCode::OK, Json(serde_json::json!({
            "jobId": job.id,
            "phase": job.phase,
            "progress": job.progress,
            "error": job.error,
            "framesReceived": job.frames_received,
            "totalFrames": job.config.total_frames,
        }))).into_response(),
        Err(_) => err(StatusCode::NOT_FOUND, "job not found").into_response(),
    }
}

pub async fn download(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let job = match state.jobs.get(id) {
        Ok(j) => j,
        Err(_) => return err(StatusCode::NOT_FOUND, "job not found").into_response(),
    };

    if job.phase != RenderPhase::Done {
        return err(StatusCode::BAD_REQUEST, "render not complete").into_response();
    }

    let output_path = match &job.output_path {
        Some(p) => std::path::PathBuf::from(p),
        None => return err(StatusCode::INTERNAL_SERVER_ERROR, "output path missing").into_response(),
    };

    let file = match tokio::fs::File::open(&output_path).await {
        Ok(f) => f,
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let content_type = match job.config.format {
        video_core::OutputFormat::Mp4 => "video/mp4",
        video_core::OutputFormat::Webm => "video/webm",
        video_core::OutputFormat::Gif => "image/gif",
    };

    let filename = format!("render-{}.{}", id, job.config.format.extension());

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\"")),
        ],
        body,
    ).into_response()
}

pub async fn list_jobs(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let jobs = state.jobs.list();
    Json(serde_json::json!({ "jobs": jobs }))
}
