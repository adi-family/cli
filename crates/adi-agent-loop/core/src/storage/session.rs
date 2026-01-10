use crate::types::{LoopConfig, LoopState, Message};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique session identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Session status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Session is actively running
    Active,
    /// Session is paused and can be resumed
    Paused,
    /// Session completed successfully
    Completed,
    /// Session failed with an error
    Failed,
    /// Session has been archived
    Archived,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Archived => "archived",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "paused" => Some(Self::Paused),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Archived)
    }
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for SessionStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("Invalid session status: {}", s))
    }
}

/// Full session data with conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,
    /// Human-readable title
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Current status
    pub status: SessionStatus,
    /// Project path this session is associated with
    pub project_path: Option<String>,
    /// System prompt used for this session
    pub system_prompt: Option<String>,
    /// Conversation messages
    pub messages: Vec<Message>,
    /// Loop configuration
    pub loop_config: LoopConfig,
    /// Current loop state
    pub loop_state: LoopState,
    /// Error message if status is Failed
    pub error_message: Option<String>,
    /// Session metadata (arbitrary JSON)
    pub metadata: serde_json::Value,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session with the given title
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            title: title.into(),
            description: None,
            status: SessionStatus::Active,
            project_path: None,
            system_prompt: None,
            messages: Vec::new(),
            loop_config: LoopConfig::default(),
            loop_state: LoopState::default(),
            error_message: None,
            metadata: serde_json::Value::Null,
            created_at: now,
            updated_at: now,
        }
    }

    /// Builder: set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Builder: set project path
    pub fn with_project_path(mut self, path: impl Into<String>) -> Self {
        self.project_path = Some(path.into());
        self
    }

    /// Builder: set system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Builder: set loop config
    pub fn with_loop_config(mut self, config: LoopConfig) -> Self {
        self.loop_config = config;
        self
    }

    /// Builder: set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// Mark session as paused
    pub fn pause(&mut self) {
        if !self.status.is_terminal() {
            self.status = SessionStatus::Paused;
            self.updated_at = Utc::now();
        }
    }

    /// Mark session as active (resume)
    pub fn resume(&mut self) {
        if self.status == SessionStatus::Paused {
            self.status = SessionStatus::Active;
            self.updated_at = Utc::now();
        }
    }

    /// Mark session as completed
    pub fn complete(&mut self) {
        self.status = SessionStatus::Completed;
        self.updated_at = Utc::now();
    }

    /// Mark session as failed with error
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = SessionStatus::Failed;
        self.error_message = Some(error.into());
        self.updated_at = Utc::now();
    }

    /// Archive the session
    pub fn archive(&mut self) {
        self.status = SessionStatus::Archived;
        self.updated_at = Utc::now();
    }

    /// Get the last user message
    pub fn last_user_message(&self) -> Option<&Message> {
        self.messages.iter().rev().find(|m| m.role() == "user")
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Calculate total tokens used
    pub fn total_tokens(&self) -> usize {
        self.messages.iter().map(|m| m.estimated_tokens()).sum()
    }
}

/// Summary view of a session (without full message history)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: SessionId,
    pub title: String,
    pub description: Option<String>,
    pub status: SessionStatus,
    pub project_path: Option<String>,
    pub message_count: usize,
    pub total_tokens: usize,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Session> for SessionSummary {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.clone(),
            title: session.title.clone(),
            description: session.description.clone(),
            status: session.status,
            project_path: session.project_path.clone(),
            message_count: session.message_count(),
            total_tokens: session.total_tokens(),
            error_message: session.error_message.clone(),
            created_at: session.created_at,
            updated_at: session.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_generation() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_status_serialization() {
        let status = SessionStatus::Active;
        assert_eq!(status.as_str(), "active");
        assert_eq!(SessionStatus::parse("active"), Some(SessionStatus::Active));
    }

    #[test]
    fn test_session_lifecycle() {
        let mut session = Session::new("Test session");
        assert_eq!(session.status, SessionStatus::Active);

        session.pause();
        assert_eq!(session.status, SessionStatus::Paused);

        session.resume();
        assert_eq!(session.status, SessionStatus::Active);

        session.complete();
        assert_eq!(session.status, SessionStatus::Completed);
        assert!(session.status.is_terminal());
    }

    #[test]
    fn test_session_fail() {
        let mut session = Session::new("Test");
        session.fail("Something went wrong");
        assert_eq!(session.status, SessionStatus::Failed);
        assert_eq!(
            session.error_message,
            Some("Something went wrong".to_string())
        );
    }

    #[test]
    fn test_session_builder() {
        let session = Session::new("My Session")
            .with_description("A test session")
            .with_project_path("/path/to/project")
            .with_system_prompt("You are helpful");

        assert_eq!(session.title, "My Session");
        assert_eq!(session.description, Some("A test session".to_string()));
        assert_eq!(session.project_path, Some("/path/to/project".to_string()));
        assert_eq!(session.system_prompt, Some("You are helpful".to_string()));
    }

    #[test]
    fn test_session_summary() {
        let session = Session::new("Test")
            .with_description("Desc")
            .with_project_path("/test");

        let summary = SessionSummary::from(&session);
        assert_eq!(summary.title, "Test");
        assert_eq!(summary.description, Some("Desc".to_string()));
        assert_eq!(summary.message_count, 0);
    }
}
