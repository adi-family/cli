mod session;
mod sqlite;

pub use session::{Session, SessionId, SessionStatus, SessionSummary};
pub use sqlite::SqliteSessionStorage;

use crate::error::Result;

/// Storage trait for session persistence
pub trait SessionStorage: Send + Sync {
    /// Create a new session
    fn create_session(&self, session: &Session) -> Result<SessionId>;

    /// Get a session by ID
    fn get_session(&self, id: &SessionId) -> Result<Session>;

    /// Update an existing session
    fn update_session(&self, session: &Session) -> Result<()>;

    /// Delete a session
    fn delete_session(&self, id: &SessionId) -> Result<()>;

    /// List all sessions, optionally filtered by project path
    fn list_sessions(&self, project_path: Option<&str>) -> Result<Vec<SessionSummary>>;

    /// List sessions by status
    fn list_sessions_by_status(&self, status: SessionStatus) -> Result<Vec<SessionSummary>>;

    /// Search sessions by query (title/description)
    fn search_sessions(&self, query: &str, limit: usize) -> Result<Vec<SessionSummary>>;

    /// Get sessions count by status
    fn get_session_counts(&self) -> Result<SessionCounts>;

    /// Archive old completed sessions
    fn archive_old_sessions(&self, older_than_days: u32) -> Result<usize>;
}

/// Session count statistics
#[derive(Debug, Clone, Default)]
pub struct SessionCounts {
    pub total: u64,
    pub active: u64,
    pub paused: u64,
    pub completed: u64,
    pub failed: u64,
    pub archived: u64,
}
