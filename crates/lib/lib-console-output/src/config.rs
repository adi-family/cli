// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Configuration for console output behavior.

use crate::{Level, OutputMode};

/// Environment variable for silk mode (JSON stream output).
pub const SILK_MODE_ENV: &str = "SILK_MODE";

/// Environment variable for disabling colors.
pub const NO_COLOR_ENV: &str = "NO_COLOR";

/// Environment variable for verbose output.
pub const VERBOSE_ENV: &str = "VERBOSE";

/// Environment variable for quiet mode (errors only).
pub const QUIET_ENV: &str = "QUIET";

/// Console output configuration.
#[derive(Debug, Clone)]
pub struct ConsoleConfig {
    /// Output mode (text or JSON stream).
    pub mode: OutputMode,
    /// Minimum level to display.
    pub min_level: Level,
    /// Whether colors are enabled (only applies to text mode).
    pub colors_enabled: bool,
}

impl ConsoleConfig {
    /// Create configuration from environment variables.
    ///
    /// Reads:
    /// - `SILK_MODE` - Set to "true" or "1" for JSON stream output
    /// - `NO_COLOR` - Set to disable colors in text mode
    /// - `VERBOSE` - Set to "true" or "1" for trace-level output
    /// - `QUIET` - Set to "true" or "1" for errors only
    pub fn from_env() -> Self {
        let mode = if is_env_truthy(SILK_MODE_ENV) {
            OutputMode::JsonStream
        } else {
            OutputMode::Text
        };

        let min_level = if is_env_truthy(QUIET_ENV) {
            Level::Error
        } else if is_env_truthy(VERBOSE_ENV) {
            Level::Trace
        } else {
            Level::Info
        };

        let colors_enabled = std::env::var(NO_COLOR_ENV).is_err();

        Self {
            mode,
            min_level,
            colors_enabled,
        }
    }

    /// Create default text mode configuration.
    pub fn text() -> Self {
        Self {
            mode: OutputMode::Text,
            min_level: Level::Info,
            colors_enabled: true,
        }
    }

    /// Create JSON stream mode configuration.
    pub fn json_stream() -> Self {
        Self {
            mode: OutputMode::JsonStream,
            min_level: Level::Trace,
            colors_enabled: false,
        }
    }

    /// Set the output mode.
    pub fn with_mode(mut self, mode: OutputMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the minimum output level.
    pub fn with_min_level(mut self, level: Level) -> Self {
        self.min_level = level;
        self
    }

    /// Enable or disable colors.
    pub fn with_colors(mut self, enabled: bool) -> Self {
        self.colors_enabled = enabled;
        self
    }
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Check if an environment variable is set to a truthy value.
fn is_env_truthy(var: &str) -> bool {
    std::env::var(var)
        .map(|v| {
            let v = v.to_lowercase();
            v == "true" || v == "1" || v == "yes" || v == "on"
        })
        .unwrap_or(false)
}
