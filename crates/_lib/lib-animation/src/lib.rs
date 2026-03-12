//! UI animation utilities - easing functions, spring physics, animation manager
//!
//! Framework-agnostic animation primitives for smooth UI transitions.

mod easing;
mod manager;
mod spring;

pub use easing::Easing;
pub use manager::{Animation, AnimationId, AnimationManager};
pub use spring::AnimatedValue;

use std::time::Duration;

/// Standard animation duration (200ms)
pub const TRANSITION_DURATION: Duration = Duration::from_millis(200);

/// Animation tick rate for smooth 60fps animations
pub const ANIMATION_TICK_MS: u64 = 16;

/// Fast animation (100ms)
pub const FAST_DURATION: Duration = Duration::from_millis(100);

/// Slow animation (300ms)
pub const SLOW_DURATION: Duration = Duration::from_millis(300);

/// Color animation helper (framework-agnostic RGBA)
#[derive(Debug, Clone, Copy, Default)]
pub struct AnimatedColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl AnimatedColor {
    /// Create from RGBA values (0.0-1.0)
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create from RGB values with full opacity
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Interpolate between two colors
    pub fn lerp(from: Self, to: Self, t: f32) -> Self {
        Self {
            r: from.r + (to.r - from.r) * t,
            g: from.g + (to.g - from.g) * t,
            b: from.b + (to.b - from.b) * t,
            a: from.a + (to.a - from.a) * t,
        }
    }

    /// Convert to RGBA array
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Create from RGBA array
    pub fn from_array(arr: [f32; 4]) -> Self {
        Self {
            r: arr[0],
            g: arr[1],
            b: arr[2],
            a: arr[3],
        }
    }
}

/// Linear interpolation between two values
pub fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

/// Clamp a value between min and max
pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}
