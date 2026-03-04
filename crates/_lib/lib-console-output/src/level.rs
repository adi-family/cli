// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Output levels for console messages.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Log/output level for console messages.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    /// Trace-level output (most verbose).
    Trace,
    /// Debug-level output.
    Debug,
    /// Informational messages.
    #[default]
    Info,
    /// Success messages (task completed).
    Success,
    /// Warning messages.
    Warn,
    /// Error messages.
    Error,
}

impl Level {
    /// Returns the string representation used in JSON output.
    pub fn as_str(&self) -> &'static str {
        match self {
            Level::Trace => "trace",
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Success => "success",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }

    /// Returns true if this level is at least as severe as `other`.
    pub fn is_at_least(&self, other: Level) -> bool {
        *self >= other
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
