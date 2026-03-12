//! Color palette types for terminal themes

use lib_misc_color::Color;

/// Complete terminal color palette (ANSI 16 colors)
#[derive(Debug, Clone)]
pub struct ColorPalette {
    pub black: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub magenta: Color,
    pub cyan: Color,
    pub white: Color,
    pub bright_black: Color,
    pub bright_red: Color,
    pub bright_green: Color,
    pub bright_yellow: Color,
    pub bright_blue: Color,
    pub bright_magenta: Color,
    pub bright_cyan: Color,
    pub bright_white: Color,
}

/// Cursor style options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorStyle {
    #[default]
    Block,
    Underline,
    Bar,
}

/// Cursor configuration
#[derive(Debug, Clone)]
pub struct CursorConfig {
    pub style: CursorStyle,
    pub color: Color,
    pub blink: bool,
    pub blink_rate: f32,
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            style: CursorStyle::Block,
            color: Color::rgba_float(1.0, 1.0, 1.0, 0.9),
            blink: true,
            blink_rate: 1.2,
        }
    }
}

/// Sidebar visual configuration
#[derive(Debug, Clone)]
pub struct SidebarTheme {
    pub background: Color,
    pub header_background: Color,
    pub session_background: Color,
    pub session_hover: Color,
    pub session_active: Color,
    pub text: Color,
    pub text_dim: Color,
    pub text_active: Color,
    pub border: Color,
    pub new_button: Color,
    pub new_button_hover: Color,
    pub new_button_text: Color,
    pub close_button: Color,
    pub close_button_hover: Color,
    pub scrollbar: Color,
    pub scrollbar_thumb: Color,
}
