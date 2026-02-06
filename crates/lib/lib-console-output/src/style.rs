// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Text styling utilities for console output.

use crate::theme;
use crate::Level;
use console::StyledObject;

/// Icons for different message levels.
pub mod icons {
    use crate::theme;
    use console::StyledObject;

    /// Green checkmark for success messages.
    pub fn success() -> StyledObject<&'static str> {
        theme::success(theme::icons::SUCCESS)
    }

    /// Red X for error messages.
    pub fn error() -> StyledObject<&'static str> {
        theme::error(theme::icons::ERROR)
    }

    /// Yellow warning sign for warnings.
    pub fn warning() -> StyledObject<&'static str> {
        theme::warning(theme::icons::WARNING)
    }

    /// Magenta info icon.
    pub fn info() -> StyledObject<&'static str> {
        theme::info(theme::icons::INFO)
    }

    /// Cyan arrow for debug.
    pub fn debug() -> StyledObject<&'static str> {
        theme::debug(theme::icons::DEBUG)
    }

    /// Dimmed dot for trace.
    pub fn trace() -> StyledObject<&'static str> {
        theme::muted(theme::icons::TRACE)
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
        Level::Trace => theme::muted("TRACE"),
        Level::Debug => theme::debug("DEBUG"),
        Level::Info => theme::info("INFO"),
        Level::Success => theme::success("OK"),
        Level::Warn => theme::warning("WARN"),
        Level::Error => theme::error("ERROR"),
    }
}

/// Style text according to level.
pub fn styled_message(level: Level, message: &str, colors_enabled: bool) -> String {
    if !colors_enabled {
        return message.to_string();
    }

    match level {
        Level::Error => theme::error(message).to_string(),
        Level::Warn => theme::warning(message).to_string(),
        Level::Success => theme::success(message).to_string(),
        Level::Info => message.to_string(),
        Level::Debug => theme::debug(message).to_string(),
        Level::Trace => theme::muted(message).to_string(),
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
