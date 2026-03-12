// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Project path handling utilities.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Wrapper for project paths with common operations.
#[derive(Debug, Clone)]
pub struct ProjectPath {
    path: PathBuf,
}

impl ProjectPath {
    /// Create a new project path from a path, canonicalizing it.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path
            .as_ref()
            .canonicalize()
            .with_context(|| format!("Failed to resolve path: {}", path.as_ref().display()))?;
        Ok(Self { path })
    }

    /// Create from current directory.
    pub fn current() -> Result<Self> {
        Self::new(".")
    }

    /// Get the underlying path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the path as a PathBuf.
    pub fn to_path_buf(&self) -> PathBuf {
        self.path.clone()
    }

    /// Get the .adi directory path.
    pub fn adi_dir(&self) -> PathBuf {
        self.path.join(".adi")
    }

    /// Check if .adi directory exists.
    pub fn has_adi(&self) -> bool {
        self.adi_dir().exists()
    }

    /// Get a subdirectory within .adi.
    pub fn adi_subdir(&self, name: &str) -> PathBuf {
        self.adi_dir().join(name)
    }

    /// Ensure .adi directory exists, creating it if needed.
    pub fn ensure_adi_dir(&self) -> Result<PathBuf> {
        let dir = self.adi_dir();
        if !dir.exists() {
            std::fs::create_dir_all(&dir)
                .with_context(|| format!("Failed to create .adi directory: {}", dir.display()))?;
        }
        Ok(dir)
    }

    /// Ensure a subdirectory within .adi exists.
    pub fn ensure_adi_subdir(&self, name: &str) -> Result<PathBuf> {
        let dir = self.adi_subdir(name);
        if !dir.exists() {
            std::fs::create_dir_all(&dir)
                .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        }
        Ok(dir)
    }
}

impl AsRef<Path> for ProjectPath {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl std::fmt::Display for ProjectPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())
    }
}
