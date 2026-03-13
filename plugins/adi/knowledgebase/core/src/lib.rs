pub mod embedder;
pub mod error;
pub mod models;
pub mod search;
pub mod service;
pub mod storage;

pub mod enums {
    pub use super::models::{ApprovalStatus, AuditAction, EdgeType, NodeType};
}

pub use error::{KnowledgebaseError, Result};
pub use models::*;
pub use service::{KnowledgebaseService, KnowledgebaseServiceAdi};
