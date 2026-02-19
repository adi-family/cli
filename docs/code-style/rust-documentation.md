# Documentation Best Practices

## Why This Rule Exists

Documentation is part of the API. Users shouldn't need to read source code to understand how to use your library. Good documentation reduces support burden, speeds up onboarding, and forces you to think about usability.

Rust's tooling makes documentation easy: write doc comments, run `cargo doc`, get a beautiful website.

## Structure

**First line**: Brief summary (appears in module/search listings)
**Body**: Detailed explanation if needed
**Sections**: Examples, Errors, Panics, Safety

```rust
/// Parses a configuration file from disk.
///
/// The configuration file must be valid TOML. Values are validated
/// against the schema defined in `ConfigSchema`.
///
/// # Examples
///
/// ```
/// let config = Config::from_file("config.toml")?;
/// assert_eq!(config.timeout, Duration::from_secs(30));
/// ```
///
/// # Errors
///
/// Returns `ConfigError::NotFound` if the file doesn't exist.
/// Returns `ConfigError::ParseError` if the TOML is malformed.
///
/// # Panics
///
/// Panics if the path contains invalid UTF-8.
pub fn from_file(path: &str) -> Result<Config, ConfigError>
```

## Required Sections

| Section | When Required |
|---------|---------------|
| `# Examples` | Complex APIs, anything non-obvious |
| `# Errors` | Functions returning `Result` |
| `# Panics` | Functions that can panic |
| `# Safety` | All `unsafe` functions |

## Code Examples

Use `?` in examples, not `unwrap()`:
```rust
/// ```
/// # fn main() -> Result<(), Error> {
/// let value = parse_thing()?;
/// # Ok(())
/// # }
/// ```
```

Examples are compiled and tested by `cargo test`.

## The Test

"Can someone use this API correctly from the documentation alone?" If no, improve the docs.
