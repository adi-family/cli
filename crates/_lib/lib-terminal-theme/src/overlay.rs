//! Overlay and shadow configuration for terminal themes

use lib_misc_color::Color;

/// Overlay and shadow configuration
#[derive(Debug, Clone)]
pub struct OverlayTheme {
    // Backdrop
    pub backdrop_color: Color,
    pub backdrop_opacity: f32,

    // Shadows
    pub shadow_color: Color,
    pub shadow_offset_x: f32,
    pub shadow_offset_y: f32,
    pub shadow_blur_small: f32,
    pub shadow_blur_medium: f32,
    pub shadow_blur_large: f32,

    // Block hover effects
    pub hover_lighten_amount: f32,
    pub skeleton_opacity: f32,
    pub table_row_alt_opacity: f32,

    // Workspace colors
    pub workspace_active_opacity: f32,
    pub workspace_hover_opacity: f32,
}

impl Default for OverlayTheme {
    fn default() -> Self {
        Self {
            // Backdrop
            backdrop_color: Color::rgba_float(0.0, 0.0, 0.0, 1.0),
            backdrop_opacity: 0.6,

            // Shadows
            shadow_color: Color::rgba_float(0.0, 0.0, 0.0, 1.0),
            shadow_offset_x: 0.0,
            shadow_offset_y: 4.0,
            shadow_blur_small: 8.0,
            shadow_blur_medium: 12.0,
            shadow_blur_large: 20.0,

            // Block hover effects
            hover_lighten_amount: 0.02,
            skeleton_opacity: 0.08,
            table_row_alt_opacity: 0.03,

            // Workspace colors
            workspace_active_opacity: 0.3,
            workspace_hover_opacity: 0.2,
        }
    }
}
