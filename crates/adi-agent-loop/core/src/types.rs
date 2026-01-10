use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum Message {
    System {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    User {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    Assistant {
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<ToolCall>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    Tool {
        tool_call_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self::System {
            content: content.into(),
            timestamp: Some(Utc::now()),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::User {
            content: content.into(),
            timestamp: Some(Utc::now()),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::Assistant {
            content: Some(content.into()),
            tool_calls: None,
            timestamp: Some(Utc::now()),
        }
    }

    pub fn assistant_with_tools(tool_calls: Vec<ToolCall>) -> Self {
        Self::Assistant {
            content: None,
            tool_calls: Some(tool_calls),
            timestamp: Some(Utc::now()),
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::Tool {
            tool_call_id: tool_call_id.into(),
            content: content.into(),
            is_error: Some(false),
            timestamp: Some(Utc::now()),
        }
    }

    pub fn tool_error(tool_call_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::Tool {
            tool_call_id: tool_call_id.into(),
            content: error.into(),
            is_error: Some(true),
            timestamp: Some(Utc::now()),
        }
    }

    pub fn content(&self) -> Option<&str> {
        match self {
            Message::System { content, .. } => Some(content),
            Message::User { content, .. } => Some(content),
            Message::Assistant { content, .. } => content.as_deref(),
            Message::Tool { content, .. } => Some(content),
        }
    }

    pub fn role(&self) -> &'static str {
        match self {
            Message::System { .. } => "system",
            Message::User { .. } => "user",
            Message::Assistant { .. } => "assistant",
            Message::Tool { .. } => "tool",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Message::Assistant {
                tool_calls: None,
                content: Some(_),
                ..
            }
        )
    }

    pub fn estimated_tokens(&self) -> usize {
        let content_len = self.content().map(|c| c.len()).unwrap_or(0);
        content_len / 4 + 10
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

impl ToolCall {
    pub fn new(name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            arguments,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub success: bool,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub duration_ms: u64,
}

impl ToolResult {
    pub fn success(
        tool_call_id: impl Into<String>,
        content: impl Into<String>,
        duration_ms: u64,
    ) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            success: true,
            content: content.into(),
            error_code: None,
            duration_ms,
        }
    }

    pub fn error(
        tool_call_id: impl Into<String>,
        error: impl Into<String>,
        code: impl Into<String>,
        duration_ms: u64,
    ) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            success: false,
            content: error.into(),
            error_code: Some(code.into()),
            duration_ms,
        }
    }

    pub fn to_message(&self) -> Message {
        if self.success {
            Message::tool_result(&self.tool_call_id, &self.content)
        } else {
            Message::tool_error(&self.tool_call_id, &self.content)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    pub max_iterations: usize,
    pub max_tokens: usize,
    pub timeout_ms: u64,
    pub max_tool_result_chars: usize,
    pub max_parallel_tools: usize,
    pub retry_on_error: bool,
    pub max_retries: usize,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            max_tokens: 100_000,
            timeout_ms: 120_000,
            max_tool_result_chars: 30_000,
            max_parallel_tools: 10,
            retry_on_error: true,
            max_retries: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopState {
    pub iteration: usize,
    pub total_tokens: usize,
    pub tool_calls_count: usize,
    pub errors_count: usize,
    pub start_time: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl Default for LoopState {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            iteration: 0,
            total_tokens: 0,
            tool_calls_count: 0,
            errors_count: 0,
            start_time: now,
            last_activity: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: DateTime<Utc>,
    pub tool: String,
    pub arguments: serde_json::Value,
    pub permission: String,
    pub result: String,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_user() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role(), "user");
        assert_eq!(msg.content(), Some("Hello"));
    }

    #[test]
    fn test_message_assistant() {
        let msg = Message::assistant("Hi there");
        assert!(msg.is_terminal());
    }

    #[test]
    fn test_message_with_tools() {
        let tool_call = ToolCall::new("read_file", serde_json::json!({"path": "/test.txt"}));
        let msg = Message::assistant_with_tools(vec![tool_call]);
        assert!(!msg.is_terminal());
    }

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("call_1", "file contents", 100);
        assert!(result.success);
        let msg = result.to_message();
        assert_eq!(msg.role(), "tool");
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("call_1", "File not found", "NOT_FOUND", 50);
        assert!(!result.success);
        assert_eq!(result.error_code, Some("NOT_FOUND".to_string()));
    }

    #[test]
    fn test_loop_config_default() {
        let config = LoopConfig::default();
        assert_eq!(config.max_iterations, 50);
        assert_eq!(config.max_tokens, 100_000);
    }
}
