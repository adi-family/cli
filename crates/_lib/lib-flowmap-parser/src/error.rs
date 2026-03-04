use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse file: {path}")]
    ParseFailed { path: String },

    #[error("Unsupported language for file: {path}")]
    UnsupportedLanguage { path: String },

    #[error("Tree-sitter error: {0}")]
    TreeSitter(String),

    #[error("Annotation error: {0}")]
    Annotation(String),
}

pub type Result<T> = std::result::Result<T, ParseError>;
