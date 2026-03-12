// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! User-level directory paths for ADI tools.

use directories::ProjectDirs;
use std::path::PathBuf;

/// User-level directory paths for ADI tools.
///
/// Platform-specific paths:
/// - Linux: `~/.local/share/adi/`
/// - macOS: `~/Library/Application Support/com.adi.adi/`
/// - Windows: `C:\Users\<User>\AppData\Roaming\adi\adi\`
pub struct AdiUserDirs;

impl AdiUserDirs {
    fn project_dirs() -> Option<ProjectDirs> {
        ProjectDirs::from("com", "adi", "adi")
    }

    /// User data directory.
    pub fn data_dir() -> Option<PathBuf> {
        Self::project_dirs().map(|dirs| dirs.data_dir().to_path_buf())
    }

    /// User config file path (`<data_dir>/config.toml`).
    pub fn config_path() -> Option<PathBuf> {
        Self::data_dir().map(|dir| dir.join("config.toml"))
    }

    /// Directory for downloaded models (`<data_dir>/models/`).
    pub fn models_dir() -> Option<PathBuf> {
        Self::data_dir().map(|dir| dir.join("models"))
    }

    /// Directory for embedding models (`<data_dir>/models/embeddings/`).
    pub fn embeddings_dir() -> Option<PathBuf> {
        Self::models_dir().map(|dir| dir.join("embeddings"))
    }

    /// Cache directory for ADI tools.
    pub fn cache_dir() -> Option<PathBuf> {
        Self::project_dirs().map(|dirs| dirs.cache_dir().to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_nested_correctly() {
        if let (Some(data), Some(models), Some(embeddings)) = (
            AdiUserDirs::data_dir(),
            AdiUserDirs::models_dir(),
            AdiUserDirs::embeddings_dir(),
        ) {
            assert!(models.starts_with(&data));
            assert!(embeddings.starts_with(&models));
        }
    }

    #[test]
    fn test_config_in_data_dir() {
        if let (Some(data), Some(config)) = (AdiUserDirs::data_dir(), AdiUserDirs::config_path()) {
            assert!(config.starts_with(&data));
            assert!(config.ends_with("config.toml"));
        }
    }

    #[test]
    fn test_dirs_consistent() {
        let data = AdiUserDirs::data_dir();
        let models = AdiUserDirs::models_dir();
        let config = AdiUserDirs::config_path();
        let cache = AdiUserDirs::cache_dir();

        assert_eq!(data.is_some(), models.is_some());
        assert_eq!(data.is_some(), config.is_some());
        assert_eq!(data.is_some(), cache.is_some());
    }
}
