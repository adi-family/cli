use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Config error: {0}")]
    Config(String),

    #[error("Forward error: {0}")]
    Forward(String),

    #[error("No backends enabled for route {0}")]
    NoBackends(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
