use serde::{Serialize, Serializer, ser::SerializeMap};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Type-erased entity ID supporting common ID types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(untagged)]
pub enum EntityId {
    Uuid(Uuid),
    Int(i64),
    String(String),
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uuid(v) => write!(f, "{v}"),
            Self::Int(v) => write!(f, "{v}"),
            Self::String(v) => write!(f, "{v}"),
        }
    }
}

impl From<Uuid> for EntityId {
    fn from(v: Uuid) -> Self {
        Self::Uuid(v)
    }
}

impl From<i64> for EntityId {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<i32> for EntityId {
    fn from(v: i32) -> Self {
        Self::Int(v as i64)
    }
}

impl From<String> for EntityId {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for EntityId {
    fn from(v: &str) -> Self {
        Self::String(v.to_owned())
    }
}

/// Trait for entities that can be placed into a [`Bucket`].
pub trait Bucketable: Serialize {
    /// Collection name for this entity type (e.g. `"tasks"`).
    fn collection() -> &'static str;

    /// The unique ID of this entity instance.
    fn entity_id(&self) -> EntityId;
}

/// Normalized entity store that deduplicates entities by `(collection, id)`.
#[derive(Debug, Clone, Default)]
pub struct Bucket {
    collections: HashMap<String, HashMap<String, Value>>,
}

impl Bucket {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an entity, returning its [`EntityId`]. Deduplicates by collection + id.
    pub fn insert<T: Bucketable>(&mut self, entity: &T) -> EntityId {
        self.try_insert(entity)
            .expect("Bucketable entity must be serializable to JSON")
    }

    /// Insert many entities, returning their IDs.
    pub fn insert_many<T: Bucketable>(&mut self, entities: &[T]) -> Vec<EntityId> {
        entities.iter().map(|e| self.insert(e)).collect()
    }

    /// Fallible insert — returns `Err` if serialization fails.
    pub fn try_insert<T: Bucketable>(&mut self, entity: &T) -> Result<EntityId, serde_json::Error> {
        let id = entity.entity_id();
        let value = serde_json::to_value(entity)?;
        self.collections
            .entry(T::collection().to_owned())
            .or_default()
            .insert(id.to_string(), value);
        Ok(id)
    }

    /// Fallible insert many.
    pub fn try_insert_many<T: Bucketable>(
        &mut self,
        entities: &[T],
    ) -> Result<Vec<EntityId>, serde_json::Error> {
        entities.iter().map(|e| self.try_insert(e)).collect()
    }

    /// Merge another bucket into this one.
    pub fn merge(&mut self, other: Bucket) {
        for (collection, entities) in other.collections {
            self.collections
                .entry(collection)
                .or_default()
                .extend(entities);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.collections.values().all(HashMap::is_empty)
    }
}

impl Serialize for Bucket {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.collections.len()))?;
        for (collection, entities) in &self.collections {
            map.serialize_entry(collection, entities)?;
        }
        map.end()
    }
}

/// Normalized API response with a `result` and optional deduplicated `bucket`.
#[derive(Debug, Clone, Serialize)]
pub struct BucketResponse<T: Serialize> {
    pub result: T,
    #[serde(skip_serializing_if = "Bucket::is_empty")]
    pub bucket: Bucket,
}

impl<T: Serialize> BucketResponse<T> {
    /// Wrap an arbitrary result with no bucket.
    pub fn new(result: T) -> Self {
        Self {
            result,
            bucket: Bucket::new(),
        }
    }

    /// Wrap a result with a pre-built bucket.
    pub fn with_bucket(result: T, bucket: Bucket) -> Self {
        Self { result, bucket }
    }
}

impl BucketResponse<EntityId> {
    /// Create a response from a single bucketable entity.
    /// Result is the entity's ID; entity goes into the bucket.
    pub fn from_item<E: Bucketable>(entity: &E) -> Self {
        let mut bucket = Bucket::new();
        let id = bucket.insert(entity);
        Self { result: id, bucket }
    }
}

impl BucketResponse<Vec<EntityId>> {
    /// Create a response from a list of bucketable entities.
    /// Result is the list of IDs; entities go into the bucket.
    pub fn from_list<E: Bucketable>(entities: &[E]) -> Self {
        let mut bucket = Bucket::new();
        let ids = bucket.insert_many(entities);
        Self {
            result: ids,
            bucket,
        }
    }
}

#[cfg(feature = "axum")]
impl<T: Serialize> axum::response::IntoResponse for BucketResponse<T> {
    fn into_response(self) -> axum::response::Response {
        axum::Json(self).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Task {
        id: i64,
        title: String,
    }

    impl Bucketable for Task {
        fn collection() -> &'static str {
            "tasks"
        }
        fn entity_id(&self) -> EntityId {
            self.id.into()
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct User {
        id: Uuid,
        name: String,
    }

    impl Bucketable for User {
        fn collection() -> &'static str {
            "users"
        }
        fn entity_id(&self) -> EntityId {
            self.id.into()
        }
    }

    #[test]
    fn insert_deduplicates() {
        let mut bucket = Bucket::new();
        let task = Task {
            id: 1,
            title: "A".into(),
        };
        bucket.insert(&task);
        bucket.insert(&task);

        let json = serde_json::to_value(&bucket).unwrap();
        let tasks = json["tasks"].as_object().unwrap();
        assert_eq!(tasks.len(), 1);
    }

    #[test]
    fn from_list_produces_ids_and_bucket() {
        let tasks = vec![
            Task { id: 1, title: "A".into() },
            Task { id: 2, title: "B".into() },
        ];
        let resp = BucketResponse::from_list(&tasks);

        assert_eq!(resp.result.len(), 2);
        assert_eq!(resp.result[0], EntityId::Int(1));
        assert_eq!(resp.result[1], EntityId::Int(2));

        let json = serde_json::to_value(&resp).unwrap();
        assert!(json["bucket"]["tasks"]["1"].is_object());
        assert!(json["bucket"]["tasks"]["2"].is_object());
    }

    #[test]
    fn from_item_produces_id_and_bucket() {
        let task = Task { id: 42, title: "X".into() };
        let resp = BucketResponse::from_item(&task);

        assert_eq!(resp.result, EntityId::Int(42));

        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["bucket"]["tasks"]["42"]["title"], "X");
    }

    #[test]
    fn empty_bucket_skipped_in_serialization() {
        let resp = BucketResponse::new("hello");
        let json = serde_json::to_value(&resp).unwrap();

        assert_eq!(json["result"], "hello");
        assert!(json.get("bucket").is_none());
    }

    #[test]
    fn mixed_id_types_in_bucket() {
        let task = Task { id: 1, title: "T".into() };
        let uid = Uuid::new_v4();
        let user = User { id: uid, name: "U".into() };

        let mut bucket = Bucket::new();
        bucket.insert(&task);
        bucket.insert(&user);

        let json = serde_json::to_value(&bucket).unwrap();
        assert!(json["tasks"]["1"].is_object());
        assert!(json["users"][uid.to_string()].is_object());
    }

    #[test]
    fn merge_combines_collections() {
        let mut a = Bucket::new();
        a.insert(&Task { id: 1, title: "A".into() });

        let mut b = Bucket::new();
        b.insert(&Task { id: 2, title: "B".into() });

        a.merge(b);

        let json = serde_json::to_value(&a).unwrap();
        let tasks = json["tasks"].as_object().unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn serialization_matches_expected_shape() {
        let tasks = vec![
            Task { id: 1, title: "Task A".into() },
            Task { id: 2, title: "Task B".into() },
        ];
        let resp = BucketResponse::from_list(&tasks);
        let json = serde_json::to_value(&resp).unwrap();

        // result is array of IDs
        assert_eq!(json["result"], serde_json::json!([1, 2]));

        // bucket has "tasks" collection with string keys
        let bucket_tasks = json["bucket"]["tasks"].as_object().unwrap();
        assert_eq!(bucket_tasks.len(), 2);
        assert_eq!(bucket_tasks["1"]["title"], "Task A");
        assert_eq!(bucket_tasks["2"]["title"], "Task B");
    }
}
