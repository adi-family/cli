//! Silk mock handler
//!
//! Simulates block-based shell sessions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use super::MessageHandler;
use crate::config::SilkScenario;

/// Silk request message (from browser)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SilkRequest {
    CreateSession {
        #[serde(default)]
        cwd: Option<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        #[serde(default)]
        shell: Option<String>,
    },
    Execute {
        session_id: String,
        command: String,
        command_id: String,
    },
    Input {
        session_id: String,
        command_id: String,
        data: String,
    },
    Resize {
        session_id: String,
        command_id: String,
        cols: u16,
        rows: u16,
    },
    Signal {
        session_id: String,
        command_id: String,
        signal: String,
    },
    CloseSession {
        session_id: String,
    },
}

/// Silk response message (to browser)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SilkResponse {
    SessionCreated {
        session_id: String,
        cwd: String,
        shell: String,
    },
    CommandStarted {
        session_id: String,
        command_id: String,
        interactive: bool,
    },
    Output {
        session_id: String,
        command_id: String,
        stream: String, // "stdout" or "stderr"
        data: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        html: Option<Vec<HtmlSpan>>,
    },
    InteractiveRequired {
        session_id: String,
        command_id: String,
        reason: String,
        pty_session_id: String,
    },
    PtyOutput {
        session_id: String,
        command_id: String,
        pty_session_id: String,
        data: String,
    },
    CommandCompleted {
        session_id: String,
        command_id: String,
        exit_code: i32,
        cwd: String,
    },
    SessionClosed {
        session_id: String,
    },
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        session_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        command_id: Option<String>,
        code: String,
        message: String,
    },
}

/// HTML span for styled output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtmlSpan {
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub styles: HashMap<String, String>,
}

/// Silk session state
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SilkSession {
    id: String,
    cwd: String,
    shell: String,
    env: HashMap<String, String>,
}

/// Silk handler for simulating block-based shell sessions
pub struct SilkHandler {
    scenario: SilkScenario,
    sessions: Arc<Mutex<HashMap<String, SilkSession>>>,
}

impl SilkHandler {
    pub fn new(scenario: SilkScenario) -> Self {
        Self {
            scenario,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn handle_request(&self, req: SilkRequest) -> Vec<SilkResponse> {
        match req {
            SilkRequest::CreateSession { cwd, env, shell } => {
                if self.scenario.fail_session {
                    return vec![SilkResponse::Error {
                        session_id: None,
                        command_id: None,
                        code: "SESSION_FAILED".to_string(),
                        message: "Session creation failed (test scenario)".to_string(),
                    }];
                }

                let session_id = Uuid::new_v4().to_string();
                let actual_cwd = cwd.unwrap_or_else(|| self.scenario.default_cwd.clone());
                let actual_shell = shell.unwrap_or_else(|| self.scenario.default_shell.clone());

                let session = SilkSession {
                    id: session_id.clone(),
                    cwd: actual_cwd.clone(),
                    shell: actual_shell.clone(),
                    env: env,
                };
                self.sessions
                    .lock()
                    .unwrap()
                    .insert(session_id.clone(), session);

                vec![SilkResponse::SessionCreated {
                    session_id,
                    cwd: actual_cwd,
                    shell: actual_shell,
                }]
            }

            SilkRequest::Execute {
                session_id,
                command,
                command_id,
            } => {
                let sessions = self.sessions.lock().unwrap();
                let session = match sessions.get(&session_id) {
                    Some(s) => s.clone(),
                    None => {
                        return vec![SilkResponse::Error {
                            session_id: Some(session_id),
                            command_id: Some(command_id),
                            code: "SESSION_NOT_FOUND".to_string(),
                            message: "Session not found".to_string(),
                        }];
                    }
                };
                drop(sessions);

                // Check if command should fail
                if self.scenario.fail_commands.contains(&command_id) {
                    return vec![
                        SilkResponse::CommandStarted {
                            session_id: session_id.clone(),
                            command_id: command_id.clone(),
                            interactive: false,
                        },
                        SilkResponse::Output {
                            session_id: session_id.clone(),
                            command_id: command_id.clone(),
                            stream: "stderr".to_string(),
                            data: "Command failed (test scenario)\n".to_string(),
                            html: None,
                        },
                        SilkResponse::CommandCompleted {
                            session_id,
                            command_id,
                            exit_code: 1,
                            cwd: session.cwd,
                        },
                    ];
                }

                // Check for interactive commands
                let interactive_commands = [
                    "vim", "nano", "less", "top", "htop", "ssh", "python", "node",
                ];
                let cmd_name = command.split_whitespace().next().unwrap_or("");

                if interactive_commands.contains(&cmd_name) {
                    let pty_session_id = Uuid::new_v4().to_string();
                    return vec![
                        SilkResponse::CommandStarted {
                            session_id: session_id.clone(),
                            command_id: command_id.clone(),
                            interactive: true,
                        },
                        SilkResponse::InteractiveRequired {
                            session_id,
                            command_id,
                            reason: format!("'{}' requires interactive terminal", cmd_name),
                            pty_session_id,
                        },
                    ];
                }

                // Get command output
                let (output, exit_code, new_cwd) = self.execute_command(&command, &session);

                let mut responses = vec![SilkResponse::CommandStarted {
                    session_id: session_id.clone(),
                    command_id: command_id.clone(),
                    interactive: false,
                }];

                if !output.is_empty() {
                    responses.push(SilkResponse::Output {
                        session_id: session_id.clone(),
                        command_id: command_id.clone(),
                        stream: if exit_code == 0 { "stdout" } else { "stderr" }.to_string(),
                        data: output,
                        html: None,
                    });
                }

                // Update cwd if changed
                if let Some(new_cwd) = &new_cwd {
                    if let Some(session) = self.sessions.lock().unwrap().get_mut(&session_id) {
                        session.cwd = new_cwd.clone();
                    }
                }

                responses.push(SilkResponse::CommandCompleted {
                    session_id,
                    command_id,
                    exit_code,
                    cwd: new_cwd.unwrap_or(session.cwd),
                });

                responses
            }

            SilkRequest::Input {
                session_id,
                command_id,
                data,
            } => {
                // For interactive commands - would route to PTY
                vec![SilkResponse::PtyOutput {
                    session_id,
                    command_id,
                    pty_session_id: "mock-pty".to_string(),
                    data,
                }]
            }

            SilkRequest::Resize { .. } => {
                // Resize doesn't produce a response
                vec![]
            }

            SilkRequest::Signal {
                session_id,
                command_id,
                signal,
            } => {
                // Simulate signal handling
                if signal == "interrupt" || signal == "kill" || signal == "terminate" {
                    let cwd = self
                        .sessions
                        .lock()
                        .unwrap()
                        .get(&session_id)
                        .map(|s| s.cwd.clone())
                        .unwrap_or_else(|| "/home/test".to_string());

                    vec![SilkResponse::CommandCompleted {
                        session_id,
                        command_id,
                        exit_code: 130, // SIGINT exit code
                        cwd,
                    }]
                } else {
                    vec![]
                }
            }

            SilkRequest::CloseSession { session_id } => {
                self.sessions.lock().unwrap().remove(&session_id);
                vec![SilkResponse::SessionClosed { session_id }]
            }
        }
    }

    fn execute_command(
        &self,
        command: &str,
        session: &SilkSession,
    ) -> (String, i32, Option<String>) {
        // Check scripted responses first
        for resp in &self.scenario.responses {
            if resp.pattern == command
                || (resp.pattern.starts_with('~') && command.contains(&resp.pattern[1..]))
            {
                return (resp.output.clone(), resp.exit_code, None);
            }
        }

        // Default command handling
        let parts: Vec<&str> = command.split_whitespace().collect();
        let cmd = parts.first().map(|s| *s).unwrap_or("");
        let args: Vec<&str> = parts.iter().skip(1).copied().collect();

        match cmd {
            "cd" => {
                let target = args.first().unwrap_or(&"~");
                let new_cwd = if target.starts_with('/') {
                    target.to_string()
                } else if *target == "~" {
                    "/home/testuser".to_string()
                } else if *target == ".." {
                    let parts: Vec<&str> = session.cwd.split('/').collect();
                    if parts.len() > 2 {
                        parts[..parts.len() - 1].join("/")
                    } else {
                        "/".to_string()
                    }
                } else {
                    format!("{}/{}", session.cwd, target)
                };
                ("".to_string(), 0, Some(new_cwd))
            }
            "pwd" => (format!("{}\n", session.cwd), 0, None),
            "whoami" => ("testuser\n".to_string(), 0, None),
            "hostname" => ("test-cocoon\n".to_string(), 0, None),
            "date" => ("Sat Jan 24 12:00:00 UTC 2026\n".to_string(), 0, None),
            "uname" => {
                if args.contains(&"-a") {
                    (
                        "Linux test-cocoon 5.15.0 #1 SMP x86_64 GNU/Linux\n".to_string(),
                        0,
                        None,
                    )
                } else {
                    ("Linux\n".to_string(), 0, None)
                }
            }
            "echo" => {
                let output = args.join(" ");
                (format!("{}\n", output), 0, None)
            }
            "cat" => {
                if args.is_empty() {
                    ("".to_string(), 0, None)
                } else {
                    (
                        format!("cat: {}: No such file or directory\n", args[0]),
                        1,
                        None,
                    )
                }
            }
            "ls" => {
                let long = args.contains(&"-l") || args.contains(&"-la") || args.contains(&"-al");
                if long {
                    (
                        "total 12\n\
                      drwxr-xr-x 2 testuser testuser 4096 Jan 24 12:00 .\n\
                      drwxr-xr-x 3 root     root     4096 Jan 24 12:00 ..\n\
                      -rw-r--r-- 1 testuser testuser  100 Jan 24 12:00 file1.txt\n\
                      -rw-r--r-- 1 testuser testuser  200 Jan 24 12:00 file2.txt\n\
                      drwxr-xr-x 2 testuser testuser 4096 Jan 24 12:00 dir1\n"
                            .to_string(),
                        0,
                        None,
                    )
                } else {
                    ("file1.txt  file2.txt  dir1\n".to_string(), 0, None)
                }
            }
            "env" => {
                let mut output = String::new();
                output.push_str(&format!("PWD={}\n", session.cwd));
                output.push_str(&format!("SHELL={}\n", session.shell));
                output.push_str("HOME=/home/testuser\n");
                output.push_str("USER=testuser\n");
                for (k, v) in &session.env {
                    output.push_str(&format!("{}={}\n", k, v));
                }
                (output, 0, None)
            }
            "true" => ("".to_string(), 0, None),
            "false" => ("".to_string(), 1, None),
            "exit" => ("".to_string(), 0, None),
            "sleep" => {
                // Just return immediately for tests
                ("".to_string(), 0, None)
            }
            "" => ("".to_string(), 0, None),
            _ => (format!("{}: command not found\n", cmd), 127, None),
        }
    }
}

impl MessageHandler for SilkHandler {
    fn handle(&self, data: &str) -> Option<String> {
        let req: SilkRequest = serde_json::from_str(data).ok()?;
        let responses = self.handle_request(req);

        if responses.is_empty() {
            return None;
        }

        // Return all responses as newline-separated JSON
        let json_responses: Vec<String> = responses
            .iter()
            .filter_map(|r| serde_json::to_string(r).ok())
            .collect();

        Some(json_responses.join("\n"))
    }

    fn channel(&self) -> &'static str {
        "silk"
    }
}
