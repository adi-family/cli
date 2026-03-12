//! Version Vectors for CRDT-based causality tracking
//!
//! Each device maintains a logical clock, and version vectors track
//! the causal history across all devices for conflict resolution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Device identifier
pub type DeviceId = Uuid;

/// Logical clock for tracking causality across distributed devices
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionVector {
    /// Map of device_id -> clock value
    pub clocks: HashMap<DeviceId, u64>,
}

impl Default for VersionVector {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionVector {
    /// Create a new empty version vector
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    /// Increment the clock for the given device
    pub fn increment(&mut self, device_id: DeviceId) {
        *self.clocks.entry(device_id).or_insert(0) += 1;
    }

    /// Get the clock value for a specific device
    pub fn clock(&self, device_id: DeviceId) -> u64 {
        *self.clocks.get(&device_id).unwrap_or(&0)
    }

    /// Returns true if self strictly happens-before other (causal ordering)
    pub fn happens_before(&self, other: &VersionVector) -> bool {
        let mut at_least_one_smaller = false;

        for (device, &clock) in &self.clocks {
            let other_clock = other.clock(*device);
            if clock > other_clock {
                return false;
            }
            if clock < other_clock {
                at_least_one_smaller = true;
            }
        }

        for (device, &clock) in &other.clocks {
            if !self.clocks.contains_key(device) && clock > 0 {
                at_least_one_smaller = true;
            }
        }

        at_least_one_smaller
    }

    /// Returns true if both vectors are concurrent (neither happens-before the other)
    pub fn concurrent_with(&self, other: &VersionVector) -> bool {
        !self.happens_before(other) && !other.happens_before(self) && self != other
    }

    /// Merge two version vectors, taking the maximum clock for each device
    pub fn merged(&self, other: &VersionVector) -> VersionVector {
        let mut result = VersionVector::new();

        for device in self.clocks.keys().chain(other.clocks.keys()) {
            result
                .clocks
                .insert(*device, self.clock(*device).max(other.clock(*device)));
        }

        result
    }

    /// Total sum of all clocks (useful for rough comparison)
    pub fn total(&self) -> u64 {
        self.clocks.values().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_vector_increment() {
        let device = Uuid::new_v4();
        let mut vv = VersionVector::new();

        assert_eq!(vv.clock(device), 0);
        vv.increment(device);
        assert_eq!(vv.clock(device), 1);
        vv.increment(device);
        assert_eq!(vv.clock(device), 2);
    }

    #[test]
    fn test_happens_before() {
        let device1 = Uuid::new_v4();
        let device2 = Uuid::new_v4();

        let mut vv1 = VersionVector::new();
        vv1.increment(device1);

        let mut vv2 = VersionVector::new();
        vv2.increment(device1);
        vv2.increment(device1);

        assert!(vv1.happens_before(&vv2));
        assert!(!vv2.happens_before(&vv1));
    }

    #[test]
    fn test_concurrent() {
        let device1 = Uuid::new_v4();
        let device2 = Uuid::new_v4();

        let mut vv1 = VersionVector::new();
        vv1.increment(device1);

        let mut vv2 = VersionVector::new();
        vv2.increment(device2);

        assert!(vv1.concurrent_with(&vv2));
        assert!(vv2.concurrent_with(&vv1));
    }

    #[test]
    fn test_merge() {
        let device1 = Uuid::new_v4();
        let device2 = Uuid::new_v4();

        let mut vv1 = VersionVector::new();
        vv1.increment(device1);
        vv1.increment(device1);

        let mut vv2 = VersionVector::new();
        vv2.increment(device2);

        let merged = vv1.merged(&vv2);
        assert_eq!(merged.clock(device1), 2);
        assert_eq!(merged.clock(device2), 1);
    }
}
