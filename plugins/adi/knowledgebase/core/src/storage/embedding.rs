use crate::error::{KnowledgebaseError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};
use uuid::Uuid;

pub struct EmbeddingStorage {
    index: Mutex<Index>,
    id_map: Mutex<IdMap>,
    data_dir: PathBuf,
    dimensions: usize,
}

struct IdMap {
    uuid_to_key: HashMap<Uuid, u64>,
    key_to_uuid: HashMap<u64, Uuid>,
    next_key: u64,
}

impl IdMap {
    fn new() -> Self {
        Self {
            uuid_to_key: HashMap::new(),
            key_to_uuid: HashMap::new(),
            next_key: 0,
        }
    }

    fn get_or_create_key(&mut self, uuid: Uuid) -> u64 {
        if let Some(&key) = self.uuid_to_key.get(&uuid) {
            return key;
        }
        let key = self.next_key;
        self.next_key += 1;
        self.uuid_to_key.insert(uuid, key);
        self.key_to_uuid.insert(key, uuid);
        key
    }

    fn get_uuid(&self, key: u64) -> Option<Uuid> {
        self.key_to_uuid.get(&key).copied()
    }

    fn remove(&mut self, uuid: Uuid) -> Option<u64> {
        if let Some(key) = self.uuid_to_key.remove(&uuid) {
            self.key_to_uuid.remove(&key);
            Some(key)
        } else {
            None
        }
    }

    fn save(&self, path: &Path) -> Result<()> {
        let data: Vec<(String, u64)> = self
            .uuid_to_key
            .iter()
            .map(|(uuid, key)| (uuid.to_string(), *key))
            .collect();
        let json = serde_json::to_string(&data)?;
        fs::write(path, json)?;
        Ok(())
    }

    fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let json = fs::read_to_string(path)?;
        let data: Vec<(String, u64)> = serde_json::from_str(&json)?;
        let mut id_map = Self::new();
        for (uuid_str, key) in data {
            if let Ok(uuid) = Uuid::parse_str(&uuid_str) {
                id_map.uuid_to_key.insert(uuid, key);
                id_map.key_to_uuid.insert(key, uuid);
                if key >= id_map.next_key {
                    id_map.next_key = key + 1;
                }
            }
        }
        Ok(id_map)
    }
}

impl EmbeddingStorage {
    pub fn open(data_dir: &Path, dimensions: usize) -> Result<Self> {
        fs::create_dir_all(data_dir)?;

        let options = IndexOptions {
            dimensions,
            metric: MetricKind::Cos,
            quantization: ScalarKind::F32,
            connectivity: 16,
            expansion_add: 128,
            expansion_search: 64,
            multi: false,
        };

        let index =
            Index::new(&options).map_err(|e| KnowledgebaseError::Embedding(e.to_string()))?;

        let index_path = data_dir.join("index.usearch");
        if index_path.exists() {
            let path_str = index_path.to_string_lossy();
            index
                .load(&path_str)
                .map_err(|e| KnowledgebaseError::Embedding(e.to_string()))?;
        }

        let id_map = IdMap::load(&data_dir.join("id_map.json"))?;

        Ok(Self {
            index: Mutex::new(index),
            id_map: Mutex::new(id_map),
            data_dir: data_dir.to_path_buf(),
            dimensions,
        })
    }

    pub fn insert(&self, uuid: Uuid, embedding: &[f32]) -> Result<()> {
        if embedding.iter().any(|v| !v.is_finite()) {
            return Err(KnowledgebaseError::Embedding(
                "embedding contains NaN or Inf values".into(),
            ));
        }

        let mut id_map = self.id_map.lock().unwrap();
        let key = id_map.get_or_create_key(uuid);
        drop(id_map);

        let index = self.index.lock().unwrap();
        let new_size = index.size() + 1;
        index
            .reserve(new_size)
            .map_err(|e| KnowledgebaseError::Embedding(e.to_string()))?;
        index
            .add(key, embedding)
            .map_err(|e| KnowledgebaseError::Embedding(e.to_string()))?;
        drop(index);

        self.save()?;
        Ok(())
    }

    pub fn delete(&self, uuid: Uuid) -> Result<()> {
        let mut id_map = self.id_map.lock().unwrap();
        let removed = id_map.remove(uuid).is_some();
        drop(id_map);

        if removed {
            // Skip `index.remove()` — usearch's C++ HNSW remove corrupts the
            // graph and causes SIGSEGV on subsequent searches. The ghost entry
            // stays in the index; `search()` filters through IdMap so ghost
            // keys are silently skipped.
            self.save()?;
        }
        Ok(())
    }

    pub fn search(&self, embedding: &[f32], limit: usize) -> Result<Vec<(Uuid, f32)>> {
        if embedding.iter().any(|v| !v.is_finite()) {
            return Err(KnowledgebaseError::Embedding(
                "search query contains NaN or Inf values".into(),
            ));
        }

        let index = self.index.lock().unwrap();
        let clamped_limit = limit.min(index.size());
        if clamped_limit == 0 {
            return Ok(Vec::new());
        }
        let results = index
            .search(embedding, clamped_limit)
            .map_err(|e| KnowledgebaseError::Embedding(e.to_string()))?;

        let id_map = self.id_map.lock().unwrap();
        let mut matches = Vec::new();
        for (key, distance) in results.keys.iter().zip(results.distances.iter()) {
            if let Some(uuid) = id_map.get_uuid(*key) {
                let score = 1.0 - distance;
                matches.push((uuid, score));
            }
        }
        Ok(matches)
    }

    fn save(&self) -> Result<()> {
        let index = self.index.lock().unwrap();
        let index_path = self.data_dir.join("index.usearch");
        let path_str = index_path.to_string_lossy();
        index
            .save(&path_str)
            .map_err(|e| KnowledgebaseError::Embedding(e.to_string()))?;
        drop(index);

        let id_map = self.id_map.lock().unwrap();
        id_map.save(&self.data_dir.join("id_map.json"))?;
        Ok(())
    }

    pub fn count(&self) -> usize {
        let index = self.index.lock().unwrap();
        index.size()
    }

    pub fn dimensions(&self) -> usize {
        self.dimensions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const DIMS: usize = 4;

    fn test_storage() -> (EmbeddingStorage, TempDir) {
        let dir = TempDir::new().unwrap();
        let storage = EmbeddingStorage::open(&dir.path().join("embed"), DIMS).unwrap();
        (storage, dir)
    }

    #[test]
    fn insert_and_search() {
        let (storage, _dir) = test_storage();
        let id = Uuid::new_v4();
        let embedding = vec![1.0, 0.0, 0.0, 0.0];
        storage.insert(id, &embedding).unwrap();

        let results = storage.search(&embedding, 5).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, id);
        assert!(results[0].1 > 0.9);
    }

    #[test]
    fn search_returns_closest() {
        let (storage, _dir) = test_storage();
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        storage.insert(id_a, &[1.0, 0.0, 0.0, 0.0]).unwrap();
        storage.insert(id_b, &[0.0, 1.0, 0.0, 0.0]).unwrap();

        let results = storage.search(&[0.9, 0.1, 0.0, 0.0], 2).unwrap();
        assert_eq!(results[0].0, id_a);
        assert!(results[0].1 > results[1].1);
    }

    #[test]
    fn delete_removes_from_search_results() {
        let (storage, _dir) = test_storage();
        let id = Uuid::new_v4();
        storage.insert(id, &[1.0, 0.0, 0.0, 0.0]).unwrap();

        storage.delete(id).unwrap();
        // Ghost entry remains in usearch index, but IdMap filter excludes it
        let results = storage.search(&[1.0, 0.0, 0.0, 0.0], 5).unwrap();
        assert!(results.iter().all(|(uuid, _)| *uuid != id));
    }

    #[test]
    fn empty_search_returns_empty() {
        let (storage, _dir) = test_storage();
        let results = storage.search(&[1.0, 0.0, 0.0, 0.0], 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn persistence_across_reopen() {
        let dir = TempDir::new().unwrap();
        let embed_dir = dir.path().join("embed");
        let id = Uuid::new_v4();

        {
            let storage = EmbeddingStorage::open(&embed_dir, DIMS).unwrap();
            storage.insert(id, &[1.0, 0.0, 0.0, 0.0]).unwrap();
        }

        let storage = EmbeddingStorage::open(&embed_dir, DIMS).unwrap();
        let results = storage.search(&[1.0, 0.0, 0.0, 0.0], 5).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, id);
    }

    #[test]
    fn dimensions_preserved() {
        let (storage, _dir) = test_storage();
        assert_eq!(storage.dimensions(), DIMS);
    }

    #[test]
    fn insert_rejects_nan_embedding() {
        let (storage, _dir) = test_storage();
        let result = storage.insert(Uuid::new_v4(), &[1.0, f32::NAN, 0.0, 0.0]);
        assert!(result.is_err());
    }

    #[test]
    fn insert_rejects_inf_embedding() {
        let (storage, _dir) = test_storage();
        let result = storage.insert(Uuid::new_v4(), &[1.0, f32::INFINITY, 0.0, 0.0]);
        assert!(result.is_err());
    }

    #[test]
    fn search_rejects_nan_query() {
        let (storage, _dir) = test_storage();
        storage.insert(Uuid::new_v4(), &[1.0, 0.0, 0.0, 0.0]).unwrap();
        let result = storage.search(&[f32::NAN, 0.0, 0.0, 0.0], 5);
        assert!(result.is_err());
    }

    #[test]
    fn delete_then_search_no_crash() {
        let (storage, _dir) = test_storage();
        let id = Uuid::new_v4();
        storage.insert(id, &[1.0, 0.0, 0.0, 0.0]).unwrap();
        storage.delete(id).unwrap();
        // Must not crash — ghost entry filtered by IdMap
        let results = storage.search(&[1.0, 0.0, 0.0, 0.0], 5).unwrap();
        assert!(results.iter().all(|(uuid, _)| *uuid != id));
    }

    // usearch v2.24.0 KNOWN BUGS (not tested here — they SIGSEGV):
    // - Index::remove() + Index::search() = use-after-free in HNSW graph
    // - Index is not thread-safe without external synchronization
    // Workarounds: skip Index::remove() in delete(), use Mutex in EmbeddingStorage.
}
