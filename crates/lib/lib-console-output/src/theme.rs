// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! ADI console theme — centralized color and icon definitions.
//!
//! All visual styling flows through this module, making it the single
//! source of truth for the ADI brand identity in the terminal.

use console::{style, StyledObject};

/// Brand color — magenta. Used for spinners, selections, interactive highlights.
pub fn brand<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).magenta()
}

/// Brand color bold — used for brand mark, prominent headers.
pub fn brand_bold<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).magenta().bold()
}

/// Info styling — magenta (brand-aligned informational messages).
pub fn info<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).magenta()
}

/// Debug styling — cyan (distinct from brand for diagnostic context).
pub fn debug<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).cyan()
}

/// Success styling — green.
pub fn success<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).green()
}

/// Warning styling — yellow.
pub fn warning<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).yellow()
}

/// Error styling — red bold.
pub fn error<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).red().bold()
}

/// Muted styling — dim text for trace-level and secondary information.
pub fn muted<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).dim()
}

/// Bold text without color.
pub fn bold<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).bold()
}

/// Unicode icons used across all console output.
pub mod icons {
    /// Brand mark.
    pub const BRAND: &str = "\u{25C6}"; // ◆

    /// Success checkmark.
    pub const SUCCESS: &str = "\u{2713}"; // ✓

    /// Error cross.
    pub const ERROR: &str = "\u{2715}"; // ✕

    /// Warning sign.
    pub const WARNING: &str = "\u{26A0}"; // ⚠

    /// Info symbol.
    pub const INFO: &str = "\u{2139}"; // ℹ

    /// Debug arrow.
    pub const DEBUG: &str = "\u{203A}"; // ›

    /// Trace dot.
    pub const TRACE: &str = "\u{00B7}"; // ·

    /// Selection cursor.
    pub const CURSOR: &str = ">";

    /// Pending circle (empty).
    pub const PENDING: &str = "\u{25CB}"; // ○

    /// In-progress circle (half).
    pub const IN_PROGRESS: &str = "\u{25D0}"; // ◐

    /// Progress bar filled block.
    pub const BAR_FILLED: &str = "\u{2588}"; // █

    /// Progress bar empty block.
    pub const BAR_EMPTY: &str = "\u{2591}"; // ░
}

/// Spinner animation frames (braille pattern).
pub const SPINNER_FRAMES: &[&str] = &[
    "\u{280B}", "\u{2819}", "\u{2839}", "\u{2838}",
    "\u{283C}", "\u{2834}", "\u{2826}", "\u{2827}",
    "\u{2807}", "\u{280F}",
]; // ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏
