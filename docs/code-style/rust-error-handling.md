# Error Handling Best Practices

## Why This Rule Exists

Errors are part of the API. A function that returns `Result<T, Box<dyn Error>>` tells callers nothing about what can go wrong. They can't handle specific cases, can't provide meaningful user messages, can't decide on retry strategies.

Good error types are specific, actionable, and preserve context. They tell callers what happened, why, and what they can do about it.

## In Practice

**Use enums for error variants:**
```rust
pub enum ConfigError {
    NotFound { path: PathBuf },
    ParseError { path: PathBuf, line: usize, message: String },
    IoError { path: PathBuf, source: std::io::Error },
}
```

**Preserve the error chain:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("failed to read config from {path}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
```

**Include actionable context:**
- File paths that failed
- Parameter values that were invalid
- Expected vs actual values

## Anti-patterns

- `Box<dyn Error>` as public error type
- `.unwrap()` in library code
- Error messages like "operation failed" without details
- Swallowing errors with `let _ = ...`

## Documentation

Document errors in rustdoc:
```rust
/// # Errors
/// Returns `ConfigError::NotFound` if the file doesn't exist.
/// Returns `ConfigError::ParseError` if the TOML is malformed.
pub fn load_config(path: &Path) -> Result<Config, ConfigError>
```

## The Test

"Given this error, can the caller take appropriate action?" If no, add more context.

"Can a user understand this error message without reading the source code?" If no, improve the message.
