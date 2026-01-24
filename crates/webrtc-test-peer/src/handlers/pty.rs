//! PTY mock handler
//!
//! Simulates PTY terminal sessions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::MessageHandler;
use crate::config::PtyScenario;

/// PTY request message (from browser)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PtyRequest {
    AttachPty {
        command: String,
        cols: u16,
        rows: u16,
        #[serde(default)]
        env: HashMap<String, String>,
    },
    PtyInput {
        session_id: String,
        data: String,
    },
    PtyResize {
        session_id: String,
        cols: u16,
        rows: u16,
    },
    PtyClose {
        session_id: String,
    },
}

/// PTY response message (to browser)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PtyResponse {
    PtyCreated { session_id: String },
    PtyOutput { session_id: String, data: String },
    PtyExited { session_id: String, exit_code: i32 },
}

/// PTY session state
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PtySession {
    id: String,
    command: String,
    cols: u16,
    rows: u16,
}

/// PTY handler for simulating terminal sessions
pub struct PtyHandler {
    scenario: PtyScenario,
    sessions: Arc<Mutex<HashMap<String, PtySession>>>,
}

impl PtyHandler {
    pub fn new(scenario: PtyScenario) -> Self {
        Self {
            scenario,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn handle_request(&self, req: PtyRequest) -> Vec<PtyResponse> {
        match req {
            PtyRequest::AttachPty {
                command,
                cols,
                rows,
                ..
            } => {
                let session_id = Uuid::new_v4().to_string();

                // Store session
                let session = PtySession {
                    id: session_id.clone(),
                    command: command.clone(),
                    cols,
                    rows,
                };
                self.sessions
                    .lock()
                    .unwrap()
                    .insert(session_id.clone(), session);

                let mut responses = vec![PtyResponse::PtyCreated {
                    session_id: session_id.clone(),
                }];

                // Send initial output based on command
                let initial_output = self.get_command_output(&command);
                if !initial_output.is_empty() {
                    responses.push(PtyResponse::PtyOutput {
                        session_id: session_id.clone(),
                        data: initial_output,
                    });
                }

                // Add shell prompt
                responses.push(PtyResponse::PtyOutput {
                    session_id,
                    data: "$ ".to_string(),
                });

                responses
            }

            PtyRequest::PtyInput { session_id, data } => {
                if !self.sessions.lock().unwrap().contains_key(&session_id) {
                    return vec![];
                }

                let mut responses = vec![];

                // Echo input if configured
                if self.scenario.echo {
                    let echo_data = if let Some(prefix) = &self.scenario.echo_prefix {
                        format!("{}{}", prefix, data)
                    } else {
                        data.clone()
                    };
                    responses.push(PtyResponse::PtyOutput {
                        session_id: session_id.clone(),
                        data: echo_data,
                    });
                }

                // Check for scripted responses
                let trimmed = data.trim();
                if trimmed.ends_with('\n') || trimmed.ends_with('\r') {
                    let cmd = trimmed.trim_end_matches(|c| c == '\n' || c == '\r');
                    let output = self.get_command_output(cmd);
                    if !output.is_empty() {
                        responses.push(PtyResponse::PtyOutput {
                            session_id: session_id.clone(),
                            data: format!("\r\n{}", output),
                        });
                    }
                    // New prompt
                    responses.push(PtyResponse::PtyOutput {
                        session_id,
                        data: "\r\n$ ".to_string(),
                    });
                }

                responses
            }

            PtyRequest::PtyResize {
                session_id,
                cols,
                rows,
            } => {
                if let Some(session) = self.sessions.lock().unwrap().get_mut(&session_id) {
                    session.cols = cols;
                    session.rows = rows;
                }
                vec![]
            }

            PtyRequest::PtyClose { session_id } => {
                self.sessions.lock().unwrap().remove(&session_id);
                vec![PtyResponse::PtyExited {
                    session_id,
                    exit_code: 0,
                }]
            }
        }
    }

    fn get_command_output(&self, command: &str) -> String {
        // Check scripted responses first
        for resp in &self.scenario.responses {
            if resp.pattern.starts_with('~') {
                // Regex match
                if let Ok(re) = regex::Regex::new(&resp.pattern[1..]) {
                    if re.is_match(command) {
                        return resp.output.clone();
                    }
                }
            } else if resp.pattern == command {
                return resp.output.clone();
            }
        }

        // Default responses for common commands
        match command {
            "whoami" => "testuser".to_string(),
            "pwd" => "/home/testuser".to_string(),
            "hostname" => "test-cocoon".to_string(),
            "echo $SHELL" => "/bin/bash".to_string(),
            "date" => "Sat Jan 24 12:00:00 UTC 2026".to_string(),
            "uname -a" => "Linux test-cocoon 5.15.0 #1 SMP x86_64 GNU/Linux".to_string(),
            cmd if cmd.starts_with("echo ") => cmd[5..].to_string(),
            cmd if cmd.starts_with("cat ") => {
                format!("cat: {}: No such file or directory", &cmd[4..])
            }
            "ls" => "file1.txt  file2.txt  dir1".to_string(),
            "ls -la" => "total 12\n\
                 drwxr-xr-x 2 testuser testuser 4096 Jan 24 12:00 .\n\
                 drwxr-xr-x 3 root     root     4096 Jan 24 12:00 ..\n\
                 -rw-r--r-- 1 testuser testuser  100 Jan 24 12:00 file1.txt\n\
                 -rw-r--r-- 1 testuser testuser  200 Jan 24 12:00 file2.txt\n\
                 drwxr-xr-x 2 testuser testuser 4096 Jan 24 12:00 dir1"
                .to_string(),
            "exit" => "".to_string(),
            _ => format!(
                "{}: command not found",
                command.split_whitespace().next().unwrap_or("")
            ),
        }
    }
}

impl MessageHandler for PtyHandler {
    fn handle(&self, data: &str) -> Option<String> {
        let req: PtyRequest = serde_json::from_str(data).ok()?;
        let responses = self.handle_request(req);

        if responses.is_empty() {
            return None;
        }

        // Return first response, queue others (in real impl would stream)
        // For simplicity, concatenate all responses
        let json_responses: Vec<String> = responses
            .iter()
            .filter_map(|r| serde_json::to_string(r).ok())
            .collect();

        Some(json_responses.join("\n"))
    }

    fn channel(&self) -> &'static str {
        "pty"
    }
}

// Add regex as optional dependency
mod regex {
    pub struct Regex(());
    impl Regex {
        pub fn new(_pattern: &str) -> Result<Self, ()> {
            // Simplified: just return error, use exact match fallback
            Err(())
        }
        pub fn is_match(&self, _text: &str) -> bool {
            false
        }
    }
}
