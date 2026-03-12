//! Spring-based physics animations

/// Animated value wrapper using spring physics
#[derive(Debug, Clone)]
pub struct AnimatedValue {
    current: f32,
    target: f32,
    velocity: f32,
    spring_constant: f32,
    damping: f32,
}

impl AnimatedValue {
    /// Create a new animated value
    pub fn new(initial: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            velocity: 0.0,
            spring_constant: 300.0,
            damping: 20.0,
        }
    }

    /// Create with custom spring parameters
    pub fn with_spring(initial: f32, spring_constant: f32, damping: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            velocity: 0.0,
            spring_constant,
            damping,
        }
    }

    /// Set the target value
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Set value immediately (no animation)
    pub fn set_immediate(&mut self, value: f32) {
        self.current = value;
        self.target = value;
        self.velocity = 0.0;
    }

    /// Get current value
    pub fn value(&self) -> f32 {
        self.current
    }

    /// Get target value
    pub fn target(&self) -> f32 {
        self.target
    }

    /// Get current velocity
    pub fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Update the animation (call each frame)
    /// dt is delta time in seconds
    pub fn update(&mut self, dt: f32) {
        let displacement = self.target - self.current;
        let spring_force = displacement * self.spring_constant;
        let damping_force = -self.velocity * self.damping;
        let acceleration = spring_force + damping_force;

        self.velocity += acceleration * dt;
        self.current += self.velocity * dt;

        // Snap to target if close enough
        if displacement.abs() < 0.01 && self.velocity.abs() < 0.01 {
            self.current = self.target;
            self.velocity = 0.0;
        }
    }

    /// Update with fixed 60fps timestep
    pub fn tick(&mut self) {
        self.update(1.0 / 60.0);
    }

    /// Check if animation is at rest
    pub fn is_at_rest(&self) -> bool {
        (self.current - self.target).abs() < 0.01 && self.velocity.abs() < 0.01
    }

    /// Configure spring stiffness (higher = faster)
    pub fn set_spring_constant(&mut self, k: f32) {
        self.spring_constant = k;
    }

    /// Configure damping (higher = less bouncy)
    pub fn set_damping(&mut self, d: f32) {
        self.damping = d;
    }
}

impl Default for AnimatedValue {
    fn default() -> Self {
        Self::new(0.0)
    }
}

/// 2D animated point using spring physics
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct AnimatedPoint {
    pub x: AnimatedValue,
    pub y: AnimatedValue,
}

#[allow(dead_code)]
impl AnimatedPoint {
    /// Create a new animated point
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x: AnimatedValue::new(x),
            y: AnimatedValue::new(y),
        }
    }

    /// Set target position
    pub fn set_target(&mut self, x: f32, y: f32) {
        self.x.set_target(x);
        self.y.set_target(y);
    }

    /// Get current position
    pub fn position(&self) -> (f32, f32) {
        (self.x.value(), self.y.value())
    }

    /// Update animation
    pub fn update(&mut self, dt: f32) {
        self.x.update(dt);
        self.y.update(dt);
    }

    /// Update with fixed timestep
    pub fn tick(&mut self) {
        self.x.tick();
        self.y.tick();
    }

    /// Check if at rest
    pub fn is_at_rest(&self) -> bool {
        self.x.is_at_rest() && self.y.is_at_rest()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spring_animation() {
        let mut value = AnimatedValue::new(0.0);
        value.set_target(100.0);

        // After many updates, should approach target
        for _ in 0..1000 {
            value.tick();
        }

        assert!((value.value() - 100.0).abs() < 0.1);
        assert!(value.is_at_rest());
    }

    #[test]
    fn test_immediate_set() {
        let mut value = AnimatedValue::new(0.0);
        value.set_immediate(50.0);
        assert_eq!(value.value(), 50.0);
        assert!(value.is_at_rest());
    }
}
