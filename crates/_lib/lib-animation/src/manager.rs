//! Animation manager for tracking multiple animations

use crate::easing::Easing;
use crate::TRANSITION_DURATION;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Unique identifier for animations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnimationId(u64);

impl AnimationId {
    /// Create a new unique animation ID
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for AnimationId {
    fn default() -> Self {
        Self::new()
    }
}

/// Single animation state
#[derive(Debug, Clone)]
pub struct Animation {
    /// Start time of the animation
    pub start: Instant,
    /// Duration of the animation
    pub duration: Duration,
    /// Easing function to use
    pub easing: Easing,
    /// Starting value
    pub from: f32,
    /// Target value
    pub to: f32,
    /// Whether animation should be removed when complete
    pub auto_remove: bool,
}

impl Animation {
    /// Create a new animation
    pub fn new(from: f32, to: f32) -> Self {
        Self {
            start: Instant::now(),
            duration: TRANSITION_DURATION,
            easing: Easing::default(),
            from,
            to,
            auto_remove: true,
        }
    }

    /// Set animation duration
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set animation duration in milliseconds
    pub fn duration_ms(mut self, ms: u64) -> Self {
        self.duration = Duration::from_millis(ms);
        self
    }

    /// Set easing function
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Set auto-remove behavior
    pub fn auto_remove(mut self, auto_remove: bool) -> Self {
        self.auto_remove = auto_remove;
        self
    }

    /// Get current progress (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        let elapsed = self.start.elapsed();
        if elapsed >= self.duration {
            1.0
        } else {
            elapsed.as_secs_f32() / self.duration.as_secs_f32()
        }
    }

    /// Get current animated value
    pub fn value(&self) -> f32 {
        let progress = self.progress();
        let eased = self.easing.apply(progress);
        self.from + (self.to - self.from) * eased
    }

    /// Check if animation is complete
    pub fn is_complete(&self) -> bool {
        self.start.elapsed() >= self.duration
    }

    /// Restart animation with new target
    pub fn retarget(&mut self, new_to: f32) {
        self.from = self.value();
        self.to = new_to;
        self.start = Instant::now();
    }

    /// Restart animation from current position
    pub fn restart(&mut self) {
        self.from = self.value();
        self.start = Instant::now();
    }
}

/// Animation manager for tracking multiple animations
#[derive(Debug, Default)]
pub struct AnimationManager {
    /// Active animations by ID
    animations: HashMap<AnimationId, Animation>,
    /// Named animations (for easy lookup by component)
    named: HashMap<String, AnimationId>,
}

impl AnimationManager {
    /// Create a new animation manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new animation, returning its ID
    pub fn start(&mut self, animation: Animation) -> AnimationId {
        let id = AnimationId::new();
        self.animations.insert(id, animation);
        id
    }

    /// Start a named animation (replaces existing with same name)
    pub fn start_named(&mut self, name: impl Into<String>, animation: Animation) -> AnimationId {
        let name = name.into();
        let id = AnimationId::new();

        // Remove old animation with same name
        if let Some(old_id) = self.named.remove(&name) {
            self.animations.remove(&old_id);
        }

        self.named.insert(name, id);
        self.animations.insert(id, animation);
        id
    }

    /// Get current value of an animation by ID
    pub fn value(&self, id: AnimationId) -> Option<f32> {
        self.animations.get(&id).map(|a| a.value())
    }

    /// Get current value of a named animation
    pub fn value_named(&self, name: &str) -> Option<f32> {
        self.named
            .get(name)
            .and_then(|id| self.animations.get(id))
            .map(|a| a.value())
    }

    /// Get value or default
    pub fn value_or(&self, id: AnimationId, default: f32) -> f32 {
        self.value(id).unwrap_or(default)
    }

    /// Get named value or default
    pub fn value_named_or(&self, name: &str, default: f32) -> f32 {
        self.value_named(name).unwrap_or(default)
    }

    /// Check if an animation is complete
    pub fn is_complete(&self, id: AnimationId) -> bool {
        self.animations
            .get(&id)
            .map(|a| a.is_complete())
            .unwrap_or(true)
    }

    /// Check if any animations are running
    pub fn has_active(&self) -> bool {
        self.animations.values().any(|a| !a.is_complete())
    }

    /// Get count of active animations
    pub fn active_count(&self) -> usize {
        self.animations
            .values()
            .filter(|a| !a.is_complete())
            .count()
    }

    /// Update animations, removing completed ones with auto_remove
    pub fn tick(&mut self) {
        self.animations
            .retain(|_, anim| !(anim.is_complete() && anim.auto_remove));

        // Also clean up named references to removed animations
        self.named.retain(|_, id| self.animations.contains_key(id));
    }

    /// Retarget an existing animation or start a new one
    pub fn animate_to(&mut self, id: AnimationId, target: f32) {
        if let Some(anim) = self.animations.get_mut(&id) {
            anim.retarget(target);
        }
    }

    /// Animate a named property to a target value
    pub fn animate_named_to(&mut self, name: &str, target: f32, default_from: f32) {
        if let Some(id) = self.named.get(name).copied() {
            if let Some(anim) = self.animations.get_mut(&id) {
                anim.retarget(target);
                return;
            }
        }

        // Start new animation
        self.start_named(name, Animation::new(default_from, target));
    }

    /// Remove an animation by ID
    pub fn remove(&mut self, id: AnimationId) {
        self.animations.remove(&id);
    }

    /// Remove a named animation
    pub fn remove_named(&mut self, name: &str) {
        if let Some(id) = self.named.remove(name) {
            self.animations.remove(&id);
        }
    }

    /// Clear all animations
    pub fn clear(&mut self) {
        self.animations.clear();
        self.named.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_progress() {
        let anim = Animation::new(0.0, 100.0);
        assert!(anim.value() >= 0.0);
        assert!(anim.value() <= 100.0);
    }

    #[test]
    fn test_animation_manager() {
        let mut manager = AnimationManager::new();
        let id = manager.start(Animation::new(0.0, 100.0));
        assert!(manager.value(id).is_some());
        assert!(manager.has_active());
    }

    #[test]
    fn test_named_animation() {
        let mut manager = AnimationManager::new();
        manager.start_named("test", Animation::new(0.0, 100.0));
        assert!(manager.value_named("test").is_some());
    }
}
