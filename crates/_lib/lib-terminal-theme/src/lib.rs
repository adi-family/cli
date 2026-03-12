//! Terminal color themes and typography - framework-agnostic
//!
//! Provides data-driven theme definitions for terminal applications.
//! No framework dependencies - use these types with any UI framework.

mod animation;
mod layout;
mod overlay;
mod palette;
mod themes;
mod typography;

pub use animation::AnimationConfig;
pub use layout::LayoutConfig;
pub use lib_misc_color::Color;
pub use overlay::OverlayTheme;
pub use palette::{ColorPalette, CursorConfig, CursorStyle, SidebarTheme};
pub use themes::Theme;
pub use typography::Typography;

/// Helper to get ANSI color from palette by index
pub fn palette_color_by_index(palette: &ColorPalette, idx: u8) -> &Color {
    match idx {
        0 => &palette.black,
        1 => &palette.red,
        2 => &palette.green,
        3 => &palette.yellow,
        4 => &palette.blue,
        5 => &palette.magenta,
        6 => &palette.cyan,
        7 => &palette.white,
        8 => &palette.bright_black,
        9 => &palette.bright_red,
        10 => &palette.bright_green,
        11 => &palette.bright_yellow,
        12 => &palette.bright_blue,
        13 => &palette.bright_magenta,
        14 => &palette.bright_cyan,
        15 => &palette.bright_white,
        _ => &palette.white,
    }
}
