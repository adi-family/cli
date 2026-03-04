// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Output modes for console rendering.

use std::fmt;
use std::str::FromStr;

/// Output mode determining how console messages are formatted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OutputMode {
    /// Human-readable text with colors and icons (default).
    #[default]
    Text,
    /// JSON stream mode for WebRTC/cloud consumption.
    /// Each message is a single-line JSON object.
    JsonStream,
}

impl OutputMode {
    /// Returns true if this is text mode.
    pub fn is_text(&self) -> bool {
        matches!(self, OutputMode::Text)
    }

    /// Returns true if this is JSON stream mode.
    pub fn is_json_stream(&self) -> bool {
        matches!(self, OutputMode::JsonStream)
    }

    /// Returns the mode name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputMode::Text => "text",
            OutputMode::JsonStream => "json_stream",
        }
    }
}

impl fmt::Display for OutputMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for OutputMode {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "json" | "json_stream" | "jsonstream" | "stream" => OutputMode::JsonStream,
            _ => OutputMode::Text,
        })
    }
}
