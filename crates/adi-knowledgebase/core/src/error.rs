use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum KnowledgebaseError {
    #[error("Node not found: {0}")]
    NodeNotFound(Uuid),

    #[error("Edge not found: {0}")]
    EdgeNotFound(Uuid),

    #[error("Orphan node detected: {0} has no edges")]
    OrphanNode(Uuid),

    #[error("Circular supersedes chain detected starting from: {0}")]
    CircularSupersedes(Uuid),

    #[error("Schema validation failed: {0}")]
    SchemaValidation(String),

    #[error("Unresolved conflict between {0} and {1}")]
    UnresolvedConflict(Uuid, Uuid),

    #[error("Duplicate node detected: similar to {0}")]
    DuplicateNode(Uuid),

    #[error("Storage error: {0}")]
    Storage(#[from] rusqlite::Error),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Migration error: {0}")]
    Migration(String),
}

pub type Result<T> = std::result::Result<T, KnowledgebaseError>;
