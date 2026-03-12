// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

use console::{style, StyledObject};

/// Green checkmark for success messages.
pub fn success_icon() -> StyledObject<&'static str> {
    style("✓").green()
}

/// Red X for error messages.
pub fn error_icon() -> StyledObject<&'static str> {
    style("✕").red()
}

/// Yellow question mark for warnings/prompts.
pub fn warning_icon() -> StyledObject<&'static str> {
    style("?").yellow()
}

/// Blue info icon.
pub fn info_icon() -> StyledObject<&'static str> {
    style("ℹ").blue()
}

/// Status indicator icons for task-like items.
#[allow(dead_code)]
pub mod status {
    use console::{style, StyledObject};

    /// Empty circle (pending/todo).
    pub fn pending() -> StyledObject<&'static str> {
        style("○").white()
    }

    /// Half circle (in progress).
    pub fn in_progress() -> StyledObject<&'static str> {
        style("◐").blue()
    }

    /// Filled circle (complete/done).
    pub fn complete() -> StyledObject<&'static str> {
        style("●").green()
    }

    /// X mark (blocked/failed).
    pub fn blocked() -> StyledObject<&'static str> {
        style("✕").red()
    }

    /// Dimmed circle (cancelled/inactive).
    pub fn cancelled() -> StyledObject<&'static str> {
        style("○").dim()
    }
}
