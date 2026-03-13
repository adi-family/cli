mod storage;
mod types;

#[cfg(test)]
mod tests;

pub use storage::CliRegistryStorage;
pub use types::*;

use anyhow::{bail, Result};

pub fn validate_platform(platform: &str) -> Result<()> {
    if platform.is_empty() {
        bail!("Platform must not be empty");
    }
    let valid = platform
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !valid {
        bail!("Invalid platform '{platform}': only alphanumeric, '_', '-' allowed");
    }
    Ok(())
}
