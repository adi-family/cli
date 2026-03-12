// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

use serde::Serialize;
use std::fmt;
use std::str::FromStr;

/// Output format for CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Markdown,
}

impl FromStr for OutputFormat {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "json" => Self::Json,
            "markdown" | "md" => Self::Markdown,
            _ => Self::Text,
        })
    }
}

impl OutputFormat {
    /// Check if this is JSON format.
    pub fn is_json(&self) -> bool {
        matches!(self, Self::Json)
    }

    /// Check if this is text format.
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text)
    }

    /// Print data in this format. Returns true if printed (for JSON), false for text.
    pub fn print_if_json<T: Serialize>(&self, data: &T) -> anyhow::Result<bool> {
        if self.is_json() {
            println!("{}", serde_json::to_string_pretty(data)?);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Json => write!(f, "json"),
            Self::Markdown => write!(f, "markdown"),
        }
    }
}

impl From<&str> for OutputFormat {
    fn from(s: &str) -> Self {
        s.parse().unwrap()
    }
}

impl From<String> for OutputFormat {
    fn from(s: String) -> Self {
        s.parse().unwrap()
    }
}
