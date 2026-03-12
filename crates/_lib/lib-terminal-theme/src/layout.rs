//! Layout configuration for terminal themes

/// Layout configuration with spacing, padding, and sizing values
#[derive(Debug, Clone)]
pub struct LayoutConfig {
    // Spacing
    pub content_padding: f32,
    pub bar_padding: f32,
    pub element_spacing: f32,
    pub small_spacing: f32,

    // Border radii
    pub border_radius: f32,
    pub pill_radius: f32,
    pub overlay_radius: f32,

    // Component sizes
    pub header_height: f32,
    pub sidebar_width: f32,
    pub scrollbar_width: f32,

    // Overlay sizes
    pub palette_width: f32,
    pub palette_entries_height: f32,
    pub toast_width: f32,
    pub context_menu_width: f32,
    pub shortcut_key_width: f32,
    pub large_output_height: f32,

    // Button padding
    pub button_padding_h: f32,
    pub button_padding_v: f32,
    pub icon_button_padding: f32,
    pub pill_button_padding_h: f32,
    pub pill_button_padding_v: f32,

    // Input padding
    pub input_padding_h: f32,
    pub input_padding_v: f32,

    // Icon sizes
    pub icon_size_small: f32,
    pub icon_size_medium: f32,
    pub icon_size_large: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            // Spacing
            content_padding: 40.0,
            bar_padding: 6.0,
            element_spacing: 12.0,
            small_spacing: 4.0,

            // Border radii
            border_radius: 6.0,
            pill_radius: 12.0,
            overlay_radius: 12.0,

            // Component sizes
            header_height: 33.0,
            sidebar_width: 200.0,
            scrollbar_width: 8.0,

            // Overlay sizes
            palette_width: 500.0,
            palette_entries_height: 300.0,
            toast_width: 300.0,
            context_menu_width: 220.0,
            shortcut_key_width: 120.0,
            large_output_height: 400.0,

            // Button padding
            button_padding_h: 12.0,
            button_padding_v: 6.0,
            icon_button_padding: 4.0,
            pill_button_padding_h: 8.0,
            pill_button_padding_v: 3.0,

            // Input padding
            input_padding_h: 12.0,
            input_padding_v: 8.0,

            // Icon sizes
            icon_size_small: 12.0,
            icon_size_medium: 14.0,
            icon_size_large: 16.0,
        }
    }
}
