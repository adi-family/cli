//! Easing functions for animations

/// Easing function type
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Easing {
    /// Linear interpolation
    Linear,
    /// Smooth ease-out (decelerating)
    #[default]
    EaseOut,
    /// Smooth ease-in (accelerating)
    EaseIn,
    /// Smooth ease-in-out (accelerate then decelerate)
    EaseInOut,
    /// Cubic ease-out for snappy feel
    EaseOutCubic,
    /// Cubic ease-in
    EaseInCubic,
    /// Cubic ease-in-out
    EaseInOutCubic,
    /// Quadratic ease-out
    EaseOutQuad,
    /// Exponential ease-out
    EaseOutExpo,
    /// Back ease-out (overshoot)
    EaseOutBack,
    /// Bouncy spring effect
    Spring,
    /// Elastic bounce
    Elastic,
}

impl Easing {
    /// Apply easing function to progress value (0.0 to 1.0)
    pub fn apply(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseOut => 1.0 - (1.0 - t).powi(2),
            Easing::EaseIn => t * t,
            Easing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Easing::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInCubic => t * t * t,
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Easing::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseOutExpo => {
                if t >= 1.0 {
                    1.0
                } else {
                    1.0 - 2.0_f32.powf(-10.0 * t)
                }
            }
            Easing::EaseOutBack => {
                let c1 = 1.70158;
                let c3 = c1 + 1.0;
                let t1 = t - 1.0;
                1.0 + c3 * t1.powi(3) + c1 * t1.powi(2)
            }
            Easing::Spring => {
                let omega = 6.0;
                let zeta = 0.7;
                1.0 - ((-zeta * omega * t).exp() * (1.0 - t))
            }
            Easing::Elastic => {
                if t == 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    let c4 = (2.0 * std::f32::consts::PI) / 3.0;
                    2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
                }
            }
        }
    }

    /// Get the inverse of an easing (ease-in becomes ease-out, etc.)
    pub fn inverse(&self) -> Self {
        match self {
            Easing::EaseIn => Easing::EaseOut,
            Easing::EaseOut => Easing::EaseIn,
            Easing::EaseInCubic => Easing::EaseOutCubic,
            Easing::EaseOutCubic => Easing::EaseInCubic,
            _ => *self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_bounds() {
        for easing in [
            Easing::Linear,
            Easing::EaseOut,
            Easing::EaseIn,
            Easing::EaseInOut,
            Easing::EaseOutCubic,
            Easing::EaseOutQuad,
        ] {
            assert_eq!(easing.apply(0.0), 0.0);
            assert!((easing.apply(1.0) - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_ease_out_starts_fast() {
        let ease_out = Easing::EaseOut;
        let linear = Easing::Linear;
        // Ease-out should be ahead of linear at t=0.25
        assert!(ease_out.apply(0.25) > linear.apply(0.25));
    }

    #[test]
    fn test_ease_in_starts_slow() {
        let ease_in = Easing::EaseIn;
        let linear = Easing::Linear;
        // Ease-in should be behind linear at t=0.25
        assert!(ease_in.apply(0.25) < linear.apply(0.25));
    }
}
