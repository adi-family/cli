//! Animation configuration for terminal themes

use std::time::Duration;

/// Animation timing and opacity configuration
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    // Timing durations
    pub transition_duration: Duration,
    pub animation_tick: Duration,
    pub toast_fade: Duration,
    pub backdrop_fade: Duration,
    pub collapse_duration: Duration,
    pub celebration_duration: Duration,
    pub git_refresh_interval: Duration,

    // Opacity values
    pub hover_opacity: f32,
    pub active_opacity: f32,
    pub disabled_opacity: f32,
    pub focus_ring_opacity: f32,

    // Glow effects
    pub glow_blur_radius: f32,
    pub glow_opacity: f32,

    // Button states
    pub pill_button_bg_opacity: f32,
    pub border_width: f32,
    pub border_width_active: f32,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            // Timing durations
            transition_duration: Duration::from_millis(200),
            animation_tick: Duration::from_millis(16),
            toast_fade: Duration::from_millis(150),
            backdrop_fade: Duration::from_millis(150),
            collapse_duration: Duration::from_millis(200),
            celebration_duration: Duration::from_millis(1500),
            git_refresh_interval: Duration::from_secs(5),

            // Opacity values
            hover_opacity: 0.15,
            active_opacity: 0.25,
            disabled_opacity: 0.5,
            focus_ring_opacity: 0.4,

            // Glow effects
            glow_blur_radius: 2.0,
            glow_opacity: 0.4,

            // Button states
            pill_button_bg_opacity: 0.2,
            border_width: 1.0,
            border_width_active: 2.0,
        }
    }
}
