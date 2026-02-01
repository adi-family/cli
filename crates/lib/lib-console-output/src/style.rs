// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Text styling utilities for console output.

use crate::Level;
use console::{style, StyledObject};

/// Icons for different message levels.
pub mod icons {
    use console::{style, StyledObject};

    /// Green checkmark for success messages.
    pub fn success() -> StyledObject<&'static str> {
        style("\u{2713}").green()
    }

    /// Red X for error messages.
    pub fn error() -> StyledObject<&'static str> {
        style("\u{2715}").red()
    }

    /// Yellow exclamation for warnings.
    pub fn warning() -> StyledObject<&'static str> {
        style("!").yellow()
    }

    /// Blue info icon.
    pub fn info() -> StyledObject<&'static str> {
        style("\u{2139}").blue()
    }

    /// Cyan arrow for debug.
    pub fn debug() -> StyledObject<&'static str> {
        style(">").cyan()
    }

    /// Dimmed dot for trace.
    pub fn trace() -> StyledObject<&'static str> {
        style(".").dim()
    }
}

/// Get the icon for a message level.
pub fn level_icon(level: Level) -> StyledObject<&'static str> {
    match level {
        Level::Trace => icons::trace(),
        Level::Debug => icons::debug(),
        Level::Info => icons::info(),
        Level::Success => icons::success(),
        Level::Warn => icons::warning(),
        Level::Error => icons::error(),
    }
}

/// Get the styled prefix text for a level.
pub fn level_prefix(level: Level) -> StyledObject<&'static str> {
    match level {
        Level::Trace => style("TRACE").dim(),
        Level::Debug => style("DEBUG").cyan(),
        Level::Info => style("INFO").blue(),
        Level::Success => style("OK").green(),
        Level::Warn => style("WARN").yellow(),
        Level::Error => style("ERROR").red().bold(),
    }
}

/// Style text according to level.
pub fn styled_message(level: Level, message: &str, colors_enabled: bool) -> String {
    if !colors_enabled {
        return message.to_string();
    }

    match level {
        Level::Error => style(message).red().to_string(),
        Level::Warn => style(message).yellow().to_string(),
        Level::Success => style(message).green().to_string(),
        Level::Info => message.to_string(),
        Level::Debug => style(message).cyan().to_string(),
        Level::Trace => style(message).dim().to_string(),
    }
}

/// Format a text mode output line.
pub fn format_text_line(level: Level, message: &str, colors_enabled: bool) -> String {
    if colors_enabled {
        format!("{} {}", level_icon(level), message)
    } else {
        format!("[{}] {}", level.as_str().to_uppercase(), message)
    }
}
