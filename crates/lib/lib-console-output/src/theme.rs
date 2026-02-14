// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! ADI console theme — centralized color and icon definitions.
//!
//! All visual styling flows through this module, making it the single
//! source of truth for the ADI brand identity in the terminal.
//!
//! The accent color is driven by the active theme (selected via `ADI_THEME`
//! env var or programmatically via [`init`]). Status colors (success, error,
//! warning) are universal across all themes.

use console::{style, StyledObject};
use std::sync::OnceLock;

/// Environment variable for theme selection.
pub const ADI_THEME_ENV: &str = "ADI_THEME";
// Note: ADI_THEME is also managed in cli/src/clienv.rs for the CLI crate.

/// Generated theme definitions from packages/theme/themes.json.
pub mod generated {
    include!("../../../../packages/theme/generated/themes.rs");
}

pub use generated::{find_theme, Theme, ThemeFonts, ThemeMode, DEFAULT_THEME, THEMES};

/// Active theme resolved at runtime.
static ACTIVE_THEME: OnceLock<&'static Theme> = OnceLock::new();

/// Active accent as ANSI 256-color index (cached from active theme).
static ACCENT_256: OnceLock<u8> = OnceLock::new();

/// Initialize the active theme by ID. Call early in main().
///
/// If not called, the theme auto-resolves from `ADI_THEME` env var
/// or falls back to the default theme on first use.
pub fn init(theme_id: &str) {
    let theme = find_theme(theme_id)
        .or_else(|| find_theme(DEFAULT_THEME))
        .expect("default theme must exist");
    let _ = ACTIVE_THEME.set(theme);
    let (r, g, b) = parse_hex(theme.dark.accent);
    let _ = ACCENT_256.set(rgb_to_ansi256(r, g, b));
}

/// Get the active theme.
pub fn active() -> &'static Theme {
    ACTIVE_THEME.get_or_init(|| {
        let theme_id = lib_env_parse::env_or(ADI_THEME_ENV, DEFAULT_THEME);
        find_theme(&theme_id)
            .or_else(|| find_theme(DEFAULT_THEME))
            .expect("default theme must exist")
    })
}

/// Get the accent color as ANSI 256-color index.
fn accent_color() -> u8 {
    *ACCENT_256.get_or_init(|| {
        let theme = active();
        let (r, g, b) = parse_hex(theme.dark.accent);
        rgb_to_ansi256(r, g, b)
    })
}

/// Brand color — accent from active theme. Used for spinners, selections, interactive highlights.
pub fn brand<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).color256(accent_color())
}

/// Brand color bold — used for brand mark, prominent headers.
pub fn brand_bold<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).color256(accent_color()).bold()
}

/// Info styling — accent from active theme (brand-aligned informational messages).
pub fn info<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).color256(accent_color())
}

/// Debug styling — cyan (distinct from brand for diagnostic context).
pub fn debug<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).cyan()
}

/// Success styling — green (universal across all themes).
pub fn success<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).green()
}

/// Warning styling — yellow (universal across all themes).
pub fn warning<D: std::fmt::Display>(val: D) -> StyledObject<D> {
    style(val).yellow()
}

/// Error styling — red bold (universal across all themes).
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

/// Parse a hex color string (e.g. "#6C5CE7") to RGB.
fn parse_hex(hex: &str) -> (u8, u8, u8) {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    (r, g, b)
}

/// Map RGB to the closest ANSI 256-color index.
fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    // Grayscale shortcut
    if r == g && g == b {
        if r < 8 {
            return 16;
        }
        if r > 248 {
            return 231;
        }
        return 232 + ((r as u16 - 8) * 24 / 247) as u8;
    }
    // Map to 6x6x6 color cube (indices 16-231)
    let ri = (r as u16 * 5 / 255) as u8;
    let gi = (g as u16 * 5 / 255) as u8;
    let bi = (b as u16 * 5 / 255) as u8;
    16 + 36 * ri + 6 * gi + bi
}
