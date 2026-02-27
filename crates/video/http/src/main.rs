use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

use lib_env_parse::{env_vars, env_opt};
use video_core::{FrameStore, JobStore};

env_vars! {
    Port => "PORT",
    FramesDir => "FRAMES_DIR",
}

mod api;

use api::create_router;

pub struct AppState {
    pub jobs: JobStore,
    pub frames: FrameStore,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("video=info".parse()?),
        )
        .init();

    let port = env_opt(EnvVar::Port.as_str())
        .and_then(|p| p.parse().ok())
        .unwrap_or(3100);

    let frames_dir = env_opt(EnvVar::FramesDir.as_str())
        .unwrap_or_else(|| {
            let dir = std::env::temp_dir().join("adi-video-frames");
            dir.to_string_lossy().to_string()
        });

    let state = Arc::new(AppState {
        jobs: JobStore::new(),
        frames: FrameStore::new(&frames_dir),
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let router = create_router(state);

    info!("Starting adi-video on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
