//! UI color abstraction layer
//!
//! Converts theme colors to Iced-compatible colors for use in components.
//! Also provides gradient utilities for subtle depth effects.

use iced::{gradient, Color, Radians};
use lib_terminal_theme::{Color as ThemeColor, Theme as AppTheme};

/// Iced-compatible UI colors derived from the app theme
#[derive(Debug, Clone, Copy)]
pub struct UiColors {
    // Backgrounds
    pub background: Color,
    pub blocks_bg: Color,
    pub block_bg: Color,
    pub input_bg: Color,
    pub system_bg: Color,

    // Borders
    pub border: Color,
    pub border_running: Color,
    pub border_success: Color,
    pub border_error: Color,
    pub border_interactive: Color,

    // Text
    pub header_text: Color,
    pub prompt_text: Color,
    pub command_text: Color,
    pub output_text: Color,
    pub muted_text: Color,

    // Status colors
    pub status_running: Color,
    pub status_success: Color,
    pub status_error: Color,
    pub status_info: Color,
    pub status_warning: Color,

    // Syntax highlighting colors
    pub syntax_string: Color,
    pub syntax_number: Color,
    pub syntax_variable: Color,
    pub syntax_path: Color,
    pub syntax_function: Color,
    pub syntax_keyword: Color,

    // Typography sizes
    pub command_size: f32,
    pub output_size: f32,
    pub hint_size: f32,
    pub header_size: f32,
    pub label_size: f32,

    // Layout - spacing
    pub content_padding: f32,
    pub bar_padding: f32,
    pub element_spacing: f32,
    pub small_spacing: f32,

    // Layout - border radii
    pub border_radius: f32,
    pub pill_radius: f32,
    pub overlay_radius: f32,

    // Layout - component sizes
    pub header_height: f32,
    pub sidebar_width: f32,
    pub scrollbar_width: f32,

    // Layout - overlay sizes
    pub palette_width: f32,
    pub palette_entries_height: f32,
    pub toast_width: f32,
    pub context_menu_width: f32,
    pub shortcut_key_width: f32,
    pub large_output_height: f32,

    // Layout - button padding
    pub button_padding_h: f32,
    pub button_padding_v: f32,
    pub icon_button_padding: f32,
    pub pill_button_padding_h: f32,
    pub pill_button_padding_v: f32,

    // Layout - input padding
    pub input_padding_h: f32,
    pub input_padding_v: f32,

    // Layout - icon sizes
    pub icon_size_small: f32,
    pub icon_size_medium: f32,
    pub icon_size_large: f32,

    // Overlay - backdrop
    pub backdrop_color: Color,
    pub backdrop_opacity: f32,

    // Overlay - shadows
    pub shadow_color: Color,
    pub shadow_offset_x: f32,
    pub shadow_offset_y: f32,
    pub shadow_blur_small: f32,
    pub shadow_blur_medium: f32,
    pub shadow_blur_large: f32,

    // Overlay - hover effects
    pub hover_lighten_amount: f32,
    pub skeleton_opacity: f32,
    pub table_row_alt_opacity: f32,
    pub workspace_active_opacity: f32,
    pub workspace_hover_opacity: f32,

    // Animation - timing (in milliseconds)
    pub transition_duration_ms: u64,
    pub animation_tick_ms: u64,
    pub toast_fade_ms: u64,
    pub backdrop_fade_ms: u64,
    pub collapse_duration_ms: u64,
    pub celebration_duration_ms: u64,
    pub git_refresh_interval_ms: u64,

    // Animation - opacity values
    pub hover_opacity: f32,
    pub active_opacity: f32,
    pub disabled_opacity: f32,
    pub focus_ring_opacity: f32,

    // Animation - glow effects
    pub glow_blur_radius: f32,
    pub glow_opacity: f32,

    // Animation - button states
    pub pill_button_bg_opacity: f32,
    pub border_width: f32,
    pub border_width_active: f32,
}

impl UiColors {
    /// Convert theme.rs Theme to UI colors
    pub fn from_theme(theme: &AppTheme) -> Self {
        let palette = &theme.palette;
        let sidebar = &theme.sidebar;
        let layout = &theme.layout;
        let overlay = &theme.overlay;
        let animation = &theme.animation;

        Self {
            background: to_iced(&theme.background),
            blocks_bg: to_iced(&sidebar.background),
            block_bg: to_iced(&sidebar.session_background),
            input_bg: to_iced(&sidebar.header_background),
            system_bg: to_iced(&sidebar.session_hover),
            border: to_iced(&sidebar.border),
            border_running: to_iced(&palette.blue),
            border_success: to_iced(&palette.green),
            border_error: to_iced(&palette.red),
            border_interactive: to_iced(&palette.magenta),
            header_text: to_iced(&theme.foreground),
            prompt_text: to_iced(&palette.bright_blue),
            command_text: to_iced(&palette.bright_white),
            output_text: to_iced(&theme.foreground),
            muted_text: to_iced(&palette.bright_black),
            status_running: to_iced(&palette.blue),
            status_success: to_iced(&palette.green),
            status_error: to_iced(&palette.red),
            status_info: to_iced(&palette.cyan),
            status_warning: to_iced(&palette.yellow),
            // Syntax highlighting: strings=green, numbers=yellow, variables=magenta,
            // paths=cyan, functions=blue, keywords=magenta
            syntax_string: to_iced(&palette.green),
            syntax_number: to_iced(&palette.yellow),
            syntax_variable: to_iced(&palette.magenta),
            syntax_path: to_iced(&palette.cyan),
            syntax_function: to_iced(&palette.blue),
            syntax_keyword: to_iced(&palette.magenta),

            // Typography
            command_size: theme.typography.command_size,
            output_size: theme.typography.output_size,
            hint_size: theme.typography.hint_size,
            header_size: theme.typography.header_size,
            label_size: theme.typography.label_size,

            // Layout - spacing
            content_padding: layout.content_padding,
            bar_padding: layout.bar_padding,
            element_spacing: layout.element_spacing,
            small_spacing: layout.small_spacing,

            // Layout - border radii
            border_radius: layout.border_radius,
            pill_radius: layout.pill_radius,
            overlay_radius: layout.overlay_radius,

            // Layout - component sizes
            header_height: layout.header_height,
            sidebar_width: layout.sidebar_width,
            scrollbar_width: layout.scrollbar_width,

            // Layout - overlay sizes
            palette_width: layout.palette_width,
            palette_entries_height: layout.palette_entries_height,
            toast_width: layout.toast_width,
            context_menu_width: layout.context_menu_width,
            shortcut_key_width: layout.shortcut_key_width,
            large_output_height: layout.large_output_height,

            // Layout - button padding
            button_padding_h: layout.button_padding_h,
            button_padding_v: layout.button_padding_v,
            icon_button_padding: layout.icon_button_padding,
            pill_button_padding_h: layout.pill_button_padding_h,
            pill_button_padding_v: layout.pill_button_padding_v,

            // Layout - input padding
            input_padding_h: layout.input_padding_h,
            input_padding_v: layout.input_padding_v,

            // Layout - icon sizes
            icon_size_small: layout.icon_size_small,
            icon_size_medium: layout.icon_size_medium,
            icon_size_large: layout.icon_size_large,

            // Overlay - backdrop
            backdrop_color: to_iced(&overlay.backdrop_color),
            backdrop_opacity: overlay.backdrop_opacity,

            // Overlay - shadows
            shadow_color: to_iced(&overlay.shadow_color),
            shadow_offset_x: overlay.shadow_offset_x,
            shadow_offset_y: overlay.shadow_offset_y,
            shadow_blur_small: overlay.shadow_blur_small,
            shadow_blur_medium: overlay.shadow_blur_medium,
            shadow_blur_large: overlay.shadow_blur_large,

            // Overlay - hover effects
            hover_lighten_amount: overlay.hover_lighten_amount,
            skeleton_opacity: overlay.skeleton_opacity,
            table_row_alt_opacity: overlay.table_row_alt_opacity,
            workspace_active_opacity: overlay.workspace_active_opacity,
            workspace_hover_opacity: overlay.workspace_hover_opacity,

            // Animation - timing (in milliseconds)
            transition_duration_ms: animation.transition_duration.as_millis() as u64,
            animation_tick_ms: animation.animation_tick.as_millis() as u64,
            toast_fade_ms: animation.toast_fade.as_millis() as u64,
            backdrop_fade_ms: animation.backdrop_fade.as_millis() as u64,
            collapse_duration_ms: animation.collapse_duration.as_millis() as u64,
            celebration_duration_ms: animation.celebration_duration.as_millis() as u64,
            git_refresh_interval_ms: animation.git_refresh_interval.as_millis() as u64,

            // Animation - opacity values
            hover_opacity: animation.hover_opacity,
            active_opacity: animation.active_opacity,
            disabled_opacity: animation.disabled_opacity,
            focus_ring_opacity: animation.focus_ring_opacity,

            // Animation - glow effects
            glow_blur_radius: animation.glow_blur_radius,
            glow_opacity: animation.glow_opacity,

            // Animation - button states
            pill_button_bg_opacity: animation.pill_button_bg_opacity,
            border_width: animation.border_width,
            border_width_active: animation.border_width_active,
        }
    }
}

/// Convert ThemeColor to iced::Color
#[inline]
pub fn to_iced(color: &ThemeColor) -> Color {
    let [r, g, b, a] = color.as_rgba_float();
    Color::from_rgba(r, g, b, a)
}

// ============================================================================
// Gradient utilities for subtle depth effects
// ============================================================================

/// Create a subtle vertical gradient for backgrounds (adds depth)
/// Goes from slightly darker at top to base color at bottom
pub fn subtle_gradient(base_color: Color, intensity: f32) -> gradient::Linear {
    let darker = Color {
        r: (base_color.r - intensity).max(0.0),
        g: (base_color.g - intensity).max(0.0),
        b: (base_color.b - intensity).max(0.0),
        a: base_color.a,
    };

    let lighter = Color {
        r: (base_color.r + intensity * 0.5).min(1.0),
        g: (base_color.g + intensity * 0.5).min(1.0),
        b: (base_color.b + intensity * 0.5).min(1.0),
        a: base_color.a,
    };

    // Vertical gradient (180 degrees = top to bottom)
    gradient::Linear::new(Radians(std::f32::consts::PI))
        .add_stop(0.0, darker)
        .add_stop(0.5, base_color)
        .add_stop(1.0, lighter)
}

/// Create a very subtle vignette-like gradient (darker at edges)
pub fn vignette_gradient(base_color: Color, intensity: f32) -> gradient::Linear {
    let darker = Color {
        r: (base_color.r - intensity).max(0.0),
        g: (base_color.g - intensity).max(0.0),
        b: (base_color.b - intensity).max(0.0),
        a: base_color.a,
    };

    // Diagonal gradient for subtle vignette effect
    gradient::Linear::new(Radians(std::f32::consts::FRAC_PI_4))
        .add_stop(0.0, darker)
        .add_stop(0.3, base_color)
        .add_stop(0.7, base_color)
        .add_stop(1.0, darker)
}
