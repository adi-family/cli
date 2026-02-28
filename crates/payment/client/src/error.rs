use thiserror::Error;

#[derive(Debug, Error)]
pub enum PaymentClientError {
    #[error("unauthorized")]
    Unauthorized,

    #[error("service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("API error (HTTP {status}): {message}")]
    Api { status: u16, message: String },

    #[error(transparent)]
    Http(#[from] reqwest::Error),
}
