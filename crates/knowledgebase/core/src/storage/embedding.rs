use crate::error::{KnowledgebaseError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};
use uuid::Uuid;

const EMBEDDING_DIM: usize = 384;

/// Embedding storage using USearch
pub struct EmbeddingStorage {
    index: Mutex<Index>,
    id_map: Mutex<IdMap>,
    data_dir: PathBuf,
}

/// Maps between UUIDs and numeric keys for USearch
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
    pub fn open(data_dir: &Path) -> Result<Self> {
        fs::create_dir_all(data_dir)?;

        let options = IndexOptions {
            dimensions: EMBEDDING_DIM,
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
        })
    }

    pub fn insert(&self, uuid: Uuid, embedding: &[f32]) -> Result<()> {
        let mut id_map = self.id_map.lock().unwrap();
        let key = id_map.get_or_create_key(uuid);
        drop(id_map);

        let index = self.index.lock().unwrap();
        index
            .add(key, embedding)
            .map_err(|e| KnowledgebaseError::Embedding(e.to_string()))?;
        drop(index);

        self.save()?;
        Ok(())
    }

    pub fn delete(&self, uuid: Uuid) -> Result<()> {
        let mut id_map = self.id_map.lock().unwrap();
        if let Some(key) = id_map.remove(uuid) {
            let index = self.index.lock().unwrap();
            index
                .remove(key)
                .map_err(|e| KnowledgebaseError::Embedding(e.to_string()))?;
            drop(index);
            self.save()?;
        }
        Ok(())
    }

    pub fn search(&self, embedding: &[f32], limit: usize) -> Result<Vec<(Uuid, f32)>> {
        let index = self.index.lock().unwrap();
        let results = index
            .search(embedding, limit)
            .map_err(|e| KnowledgebaseError::Embedding(e.to_string()))?;

        let id_map = self.id_map.lock().unwrap();
        let mut matches = Vec::new();
        for (key, distance) in results.keys.iter().zip(results.distances.iter()) {
            if let Some(uuid) = id_map.get_uuid(*key) {
                // Convert distance to similarity score (1 - distance for cosine)
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
}
