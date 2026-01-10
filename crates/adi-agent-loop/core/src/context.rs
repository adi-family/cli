use crate::types::{LoopConfig, Message};

pub struct ContextManager {
    messages: Vec<Message>,
    config: LoopConfig,
}

impl ContextManager {
    pub fn new(config: LoopConfig) -> Self {
        Self {
            messages: Vec::new(),
            config,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn messages_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }

    pub fn total_tokens(&self) -> usize {
        self.messages.iter().map(|m| m.estimated_tokens()).sum()
    }

    pub fn compact_if_needed(&mut self) {
        let current_tokens = self.total_tokens();
        let threshold = (self.config.max_tokens as f64 * 0.8) as usize;

        if current_tokens > threshold {
            self.apply_sliding_window();
        }
    }

    fn apply_sliding_window(&mut self) {
        if self.messages.is_empty() {
            return;
        }

        let mut result = Vec::new();
        let mut tokens = 0;
        let limit = self.config.max_tokens;

        if matches!(self.messages.first(), Some(Message::System { .. })) {
            let system_msg = self.messages[0].clone();
            tokens += system_msg.estimated_tokens();
            result.push(system_msg);
        }

        for msg in self.messages[1..].iter().rev() {
            let msg_tokens = msg.estimated_tokens();
            if tokens + msg_tokens > limit {
                break;
            }
            result.insert(if result.is_empty() { 0 } else { 1 }, msg.clone());
            tokens += msg_tokens;
        }

        self.messages = result;
    }

    pub fn truncate_tool_result(content: &str, max_chars: usize) -> String {
        if content.len() <= max_chars {
            return content.to_string();
        }

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        if total_lines <= 1 {
            let half = max_chars / 2;
            return format!(
                "{}...[truncated]...{}",
                &content[..half],
                &content[content.len() - half..]
            );
        }

        let head_count = 25.min(total_lines / 2);
        let tail_count = 25.min(total_lines - head_count);

        let head: String = lines[..head_count].join("\n");
        let tail: String = lines[total_lines - tail_count..].join("\n");

        format!(
            "[Lines 1-{} of {}]\n{}\n...\n[Lines {}-{} of {}]\n{}",
            head_count,
            total_lines,
            head,
            total_lines - tail_count + 1,
            total_lines,
            total_lines,
            tail
        )
    }

    pub fn set_messages(&mut self, messages: Vec<Message>) {
        self.messages = messages;
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagePriority {
    System,
    CurrentRequest,
    RecentToolResult,
    PreviousUserMessage,
    OldToolResult,
    IntermediateReasoning,
}

impl MessagePriority {
    pub fn for_message(msg: &Message, index: usize, total: usize) -> Self {
        match msg {
            Message::System { .. } => Self::System,
            Message::User { .. } => {
                if index >= total.saturating_sub(3) {
                    Self::CurrentRequest
                } else {
                    Self::PreviousUserMessage
                }
            }
            Message::Tool { .. } => {
                if index >= total.saturating_sub(5) {
                    Self::RecentToolResult
                } else {
                    Self::OldToolResult
                }
            }
            Message::Assistant { .. } => Self::IntermediateReasoning,
        }
    }

    pub fn weight(&self) -> u8 {
        match self {
            Self::System => 100,
            Self::CurrentRequest => 90,
            Self::RecentToolResult => 70,
            Self::PreviousUserMessage => 50,
            Self::OldToolResult => 30,
            Self::IntermediateReasoning => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_manager_add_message() {
        let config = LoopConfig::default();
        let mut manager = ContextManager::new(config);

        manager.add_message(Message::system("You are a helpful assistant"));
        manager.add_message(Message::user("Hello"));

        assert_eq!(manager.len(), 2);
    }

    #[test]
    fn test_truncate_short_content() {
        let content = "Short content";
        let result = ContextManager::truncate_tool_result(content, 100);
        assert_eq!(result, content);
    }

    #[test]
    fn test_truncate_long_content() {
        let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n".repeat(20);
        let result = ContextManager::truncate_tool_result(&content, 100);
        assert!(result.contains("[Lines"));
        assert!(result.contains("..."));
    }

    #[test]
    fn test_total_tokens() {
        let config = LoopConfig::default();
        let mut manager = ContextManager::new(config);

        manager.add_message(Message::user("Hello world"));
        let tokens = manager.total_tokens();
        assert!(tokens > 0);
    }

    #[test]
    fn test_message_priority() {
        let priority = MessagePriority::for_message(&Message::system("test"), 0, 10);
        assert_eq!(priority, MessagePriority::System);
        assert_eq!(priority.weight(), 100);
    }
}
