use std::fmt;

/// i18n-specific errors
#[derive(Debug)]
pub enum I18nError {
    /// Language not found
    LanguageNotFound(String),
    /// Failed to parse Fluent resource
    FluentParseError(String),
    /// Service registry error
    ServiceRegistryError(String),
    /// Invalid language code
    InvalidLanguageCode(String),
    /// Failed to invoke service
    ServiceInvokeError(String),
}

impl fmt::Display for I18nError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            I18nError::LanguageNotFound(lang) => write!(f, "Language not found: {}", lang),
            I18nError::FluentParseError(msg) => write!(f, "Failed to parse Fluent: {}", msg),
            I18nError::ServiceRegistryError(msg) => {
                write!(f, "Service registry error: {}", msg)
            }
            I18nError::InvalidLanguageCode(code) => write!(f, "Invalid language code: {}", code),
            I18nError::ServiceInvokeError(msg) => {
                write!(f, "Failed to invoke service: {}", msg)
            }
        }
    }
}

impl std::error::Error for I18nError {}

pub type Result<T> = std::result::Result<T, I18nError>;
