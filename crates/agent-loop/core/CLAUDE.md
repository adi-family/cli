adi-agent-loop-core, rust, agent-loop, llm-agents, tool-use, sessions

## Overview
- Core library for autonomous LLM agent loops
- Iterative: reasoning → tool calls → observation → repeat
- Persistent sessions with SQLite storage

## Architecture
- **Message Types**: User, Assistant, Tool result messages
- **Tool Registry**: Available tools with JSON schemas
- **Executor**: Runs tool calls, handles errors
- **Loop Controller**: Manages iterations, limits, interrupts
- **Permission System**: auto/ask/deny levels
- **Context Management**: Sliding window, truncation
- **Session Storage**: SQLite-backed persistence for pause/resume

## Key Types
- `Message` - Conversation message (User/Assistant/Tool)
- `Tool` - Tool definition with name, description, schema
- `ToolCall` - Tool invocation with id, name, arguments
- `ToolResult` - Tool execution result or error
- `Permission` - Permission level (Auto/Ask/Deny)
- `LoopConfig` - Configuration for loop limits
- `Session` - Persistent session with messages, state, metadata
- `SessionId` - Unique session identifier (UUID)
- `SessionStatus` - Active/Paused/Completed/Failed/Archived
- `SessionStorage` - Trait for session persistence
- `SqliteSessionStorage` - SQLite implementation

## Session Storage
```rust
// Create storage
let storage = SqliteSessionStorage::open(Path::new("sessions.db"))?;

// Create and save session
let mut session = Session::new("My Task")
    .with_project_path("/path/to/project")
    .with_system_prompt("You are helpful");
session.add_message(Message::user("Hello"));
let id = storage.create_session(&session)?;

// Retrieve and resume
let session = storage.get_session(&id)?;

// List sessions
let sessions = storage.list_sessions(Some("/path/to/project"))?;
let active = storage.list_sessions_by_status(SessionStatus::Active)?;
```

## Usage
```rust
let agent = AgentLoop::new(config)
    .with_tool(read_file_tool)
    .with_tool(write_file_tool);
let response = agent.run("Help me refactor this code").await?;
```
