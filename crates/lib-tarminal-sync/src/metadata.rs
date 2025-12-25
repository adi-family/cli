//! Sync metadata for conflict resolution
//!
//! Every syncable entity carries metadata for CRDT-based merging.

use crate::{DeviceId, VersionVector};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata attached to every syncable entity for conflict resolution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncMetadata {
    /// When the entity was first created
    pub created_at: DateTime<Utc>,

    /// When the entity was last modified
    pub modified_at: DateTime<Utc>,

    /// Version vector tracking causal history
    pub version: VersionVector,

    /// Device that originally created this entity
    pub origin_device_id: DeviceId,

    /// Soft delete marker (tombstone)
    pub tombstone: bool,
}

impl SyncMetadata {
    /// Create new metadata for a new entity
    pub fn new(device_id: DeviceId) -> Self {
        let now = Utc::now();
        let mut version = VersionVector::new();
        version.increment(device_id);

        Self {
            created_at: now,
            modified_at: now,
            version,
            origin_device_id: device_id,
            tombstone: false,
        }
    }

    /// Update metadata when entity is modified
    pub fn touch(&mut self, device_id: DeviceId) {
        self.modified_at = Utc::now();
        self.version.increment(device_id);
    }

    /// Mark as deleted without removing from storage
    pub fn mark_deleted(&mut self, device_id: DeviceId) {
        self.tombstone = true;
        self.touch(device_id);
    }

    /// Merge metadata from two concurrent versions
    pub fn merged(&self, other: &SyncMetadata) -> SyncMetadata {
        SyncMetadata {
            created_at: self.created_at.min(other.created_at),
            modified_at: self.modified_at.max(other.modified_at),
            version: self.version.merged(&other.version),
            origin_device_id: self.origin_device_id,
            tombstone: self.tombstone || other.tombstone,
        }
    }

    /// Check if this version happens-before another
    pub fn happens_before(&self, other: &SyncMetadata) -> bool {
        self.version.happens_before(&other.version)
    }

    /// Check if this version is concurrent with another
    pub fn concurrent_with(&self, other: &SyncMetadata) -> bool {
        self.version.concurrent_with(&other.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_metadata_new() {
        let device = Uuid::new_v4();
        let meta = SyncMetadata::new(device);

        assert_eq!(meta.origin_device_id, device);
        assert!(!meta.tombstone);
        assert_eq!(meta.version.clock(device), 1);
    }

    #[test]
    fn test_metadata_touch() {
        let device = Uuid::new_v4();
        let mut meta = SyncMetadata::new(device);
        let original_time = meta.modified_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        meta.touch(device);

        assert!(meta.modified_at > original_time);
        assert_eq!(meta.version.clock(device), 2);
    }

    #[test]
    fn test_metadata_merge() {
        let device1 = Uuid::new_v4();
        let device2 = Uuid::new_v4();

        let mut meta1 = SyncMetadata::new(device1);
        meta1.touch(device1);

        let meta2 = SyncMetadata::new(device2);

        let merged = meta1.merged(&meta2);
        assert_eq!(merged.version.clock(device1), 2);
        assert_eq!(merged.version.clock(device2), 1);
    }
}
