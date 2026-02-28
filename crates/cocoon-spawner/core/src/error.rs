/// Errors that can occur during cocoon spawner operations.
#[derive(Debug, thiserror::Error)]
pub enum SpawnerError {
    #[error("no setup tokens available")]
    TokensExhausted,

    #[error("unknown cocoon kind: {kind}")]
    UnknownKind { kind: String },

    #[error("concurrency limit reached (max {max})")]
    ConcurrencyLimit { max: usize },

    #[error("container not found: {container_id}")]
    ContainerNotFound { container_id: String },

    #[error("docker error: {0}")]
    Docker(#[from] bollard::errors::Error),

    #[error("signaling connection failed: {0}")]
    SignalingConnection(String),

    #[error("websocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("config error: {0}")]
    Config(String),
}
