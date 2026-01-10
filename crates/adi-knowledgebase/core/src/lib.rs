pub mod error;
pub mod search;
pub mod storage;
pub mod types;

pub use error::{KnowledgebaseError, Result};
pub use search::{ConflictDetector, DuplicateDetector, SearchConfig, SearchEngine};
pub use storage::Storage;
pub use types::*;

use lib_embed::{Embedder, PluginEmbedder};
#[cfg(feature = "fastembed")]
use lib_embed::FastEmbedder;
use lib_plugin_host::ServiceRegistry;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

/// Knowledgebase instance
pub struct Knowledgebase {
    storage: Storage,
    embedder: Arc<dyn Embedder>,
    data_dir: PathBuf,
}

impl Knowledgebase {
    /// Open knowledgebase with plugin support.
    /// Requires a ServiceRegistry with adi.embed registered.
    ///
    /// Uses the adi.embed plugin for embeddings (much smaller binary).
    /// Install with: `adi plugin install adi.embed`
    pub async fn open_with_plugins(
        data_dir: &Path,
        service_registry: Arc<ServiceRegistry>,
    ) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let storage = Storage::open(data_dir)?;
        let embedder = PluginEmbedder::new(service_registry)
            .map_err(|e| KnowledgebaseError::Embedding(e.to_string()))?;

        Ok(Self {
            storage,
            embedder: Arc::new(embedder),
            data_dir: data_dir.to_path_buf(),
        })
    }

    /// Open or create a knowledgebase at the given path.
    ///
    /// Requires the `fastembed` feature on lib-embed for local embeddings.
    #[deprecated(note = "Use open_with_plugins instead for smaller binary size")]
    #[cfg(feature = "fastembed")]
    pub async fn open(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let storage = Storage::open(data_dir)?;
        let embedder =
            FastEmbedder::new().map_err(|e| KnowledgebaseError::Embedding(e.to_string()))?;

        Ok(Self {
            storage,
            embedder: Arc::new(embedder),
            data_dir: data_dir.to_path_buf(),
        })
    }

    /// Open knowledgebase from default location.
    ///
    /// Requires the `fastembed` feature on lib-embed for local embeddings.
    #[deprecated(note = "Use open_with_plugins instead for smaller binary size")]
    #[cfg(feature = "fastembed")]
    pub async fn open_default() -> Result<Self> {
        let data_dir = default_data_dir();
        #[allow(deprecated)]
        Self::open(&data_dir).await
    }

    /// Generate embedding for text
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let texts = [text];
        let embeddings = self
            .embedder
            .embed(&texts)
            .map_err(|e| KnowledgebaseError::Embedding(e.to_string()))?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| KnowledgebaseError::Embedding("No embedding generated".to_string()))
    }

    /// Add knowledge from user statement
    pub async fn add_from_user(
        &self,
        user_statement: &str,
        derived_knowledge: &str,
        node_type: NodeType,
    ) -> Result<Node> {
        let source = KnowledgeSource::User {
            statement: user_statement.to_string(),
        };
        let node = Node::new(
            node_type,
            derived_knowledge.to_string(),
            derived_knowledge.to_string(),
            source,
            Confidence::MEDIUM,
        );

        self.add_node(node).await
    }

    /// Add a node to the knowledgebase
    pub async fn add_node(&self, node: Node) -> Result<Node> {
        let embedding = self.embed(&node.embedding_content())?;

        // Check for duplicates
        let detector = DuplicateDetector::new(&self.storage, 0.95);
        let duplicates = detector.find_duplicates(&embedding).await?;
        if let Some((dup_id, _)) = duplicates.first() {
            return Err(KnowledgebaseError::DuplicateNode(*dup_id));
        }

        // Store node with embedding
        self.storage.store_node(&node, &embedding).await?;

        // Auto-detect related nodes
        self.auto_link(&node, &embedding).await?;

        Ok(node)
    }

    /// Query the knowledgebase
    pub async fn query(&self, question: &str) -> Result<Vec<SearchResult>> {
        let embedding = self.embed(question)?;
        let engine = SearchEngine::new(&self.storage, SearchConfig::default());
        engine.search(&embedding).await
    }

    /// Get subgraph for agent consumption
    pub async fn query_subgraph(&self, question: &str) -> Result<Subgraph> {
        let embedding = self.embed(question)?;
        let engine = SearchEngine::new(&self.storage, SearchConfig::default());
        engine.get_subgraph(&embedding).await
    }

    /// Approve a node (set confidence to 1.0)
    pub fn approve(&self, node_id: Uuid) -> Result<()> {
        self.storage
            .graph
            .update_confidence(node_id, Confidence::APPROVED)
    }

    /// Get a node by ID
    pub fn get_node(&self, id: Uuid) -> Result<Option<Node>> {
        self.storage.get_node(id)
    }

    /// Delete a node
    pub fn delete_node(&self, id: Uuid) -> Result<()> {
        self.storage.delete_node(id)
    }

    /// Add an edge between nodes
    pub fn add_edge(
        &self,
        from_id: Uuid,
        to_id: Uuid,
        edge_type: EdgeType,
        weight: f32,
    ) -> Result<Edge> {
        let edge = Edge::new(from_id, to_id, edge_type, weight);
        self.storage.insert_edge(&edge)?;
        Ok(edge)
    }

    /// Get conflicts
    pub fn get_conflicts(&self) -> Result<Vec<(Node, Node)>> {
        self.storage.find_conflicts()
    }

    /// Get orphan nodes
    pub fn get_orphans(&self) -> Result<Vec<Node>> {
        let orphan_ids = self.storage.find_orphans()?;
        self.storage.get_nodes(&orphan_ids)
    }

    /// Auto-link node to related nodes
    async fn auto_link(&self, node: &Node, embedding: &[f32]) -> Result<()> {
        let similar = self.storage.find_similar(embedding, 5)?;

        for (related_id, score) in similar {
            if related_id == node.id {
                continue;
            }
            if score > 0.7 {
                let edge = Edge::new(node.id, related_id, EdgeType::RelatedTo, score);
                self.storage.insert_edge(&edge)?;
            }
        }

        Ok(())
    }

    /// Get data directory
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Get storage reference
    pub fn storage(&self) -> &Storage {
        &self.storage
    }
}

/// Default data directory for knowledgebase
pub fn default_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("adi")
        .join("knowledgebase")
}
