// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Core console abstraction that handles all output.

use crate::{
    style::format_text_line, ConsoleConfig, Level, MessageBuilder, OutputMessage, OutputMode,
};
use serde::Serialize;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

/// Global console instance.
static CONSOLE: OnceLock<Console> = OnceLock::new();

/// Flag indicating if global console is initialized.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Console output handler.
#[derive(Debug)]
pub struct Console {
    config: ConsoleConfig,
}

impl Console {
    /// Create a new console with the given configuration.
    pub fn new(config: ConsoleConfig) -> Self {
        Self { config }
    }

    /// Create console from environment configuration.
    pub fn from_env() -> Self {
        Self::new(ConsoleConfig::from_env())
    }

    /// Get the current configuration.
    pub fn config(&self) -> &ConsoleConfig {
        &self.config
    }

    /// Get the current output mode.
    pub fn mode(&self) -> OutputMode {
        self.config.mode
    }

    /// Check if a level should be displayed.
    pub fn should_output(&self, level: Level) -> bool {
        level.is_at_least(self.config.min_level)
    }

    /// Output a message at the given level.
    pub fn output(&self, level: Level, message: &str) {
        if !self.should_output(level) {
            return;
        }

        let line = match self.config.mode {
            OutputMode::Text => format_text_line(level, message, self.config.colors_enabled),
            OutputMode::JsonStream => OutputMessage::new(level, message).to_json(),
        };

        self.write_line(level, &line);
    }

    /// Output a structured message.
    pub fn output_message(&self, msg: &OutputMessage) {
        if !self.should_output(msg.level) {
            return;
        }

        let line = match self.config.mode {
            OutputMode::Text => {
                format_text_line(msg.level, &msg.message, self.config.colors_enabled)
            }
            OutputMode::JsonStream => msg.to_json(),
        };

        self.write_line(msg.level, &line);
    }

    /// Create a message builder for this console.
    pub fn message(&self, level: Level, message: impl Into<String>) -> MessageBuilder {
        MessageBuilder::new(level, message)
    }

    /// Output raw data as JSON (for structured output regardless of mode).
    pub fn output_data<T: Serialize>(&self, data: &T) {
        let json = match serde_json::to_string(data) {
            Ok(j) => j,
            Err(e) => {
                self.output(Level::Error, &format!("Failed to serialize data: {}", e));
                return;
            }
        };
        self.write_line(Level::Info, &json);
    }

    /// Output raw data as pretty JSON (only in text mode).
    pub fn output_data_pretty<T: Serialize>(&self, data: &T) {
        match self.config.mode {
            OutputMode::Text => {
                let json = match serde_json::to_string_pretty(data) {
                    Ok(j) => j,
                    Err(e) => {
                        self.output(Level::Error, &format!("Failed to serialize data: {}", e));
                        return;
                    }
                };
                println!("{}", json);
            }
            OutputMode::JsonStream => self.output_data(data),
        }
    }

    /// Write a line to the appropriate output stream.
    fn write_line(&self, level: Level, line: &str) {
        let is_error = matches!(level, Level::Error);

        if is_error {
            let _ = writeln!(io::stderr(), "{}", line);
        } else {
            let _ = writeln!(io::stdout(), "{}", line);
        }
    }

    // Convenience methods for common levels

    /// Output a trace message.
    pub fn trace(&self, message: &str) {
        self.output(Level::Trace, message);
    }

    /// Output a debug message.
    pub fn debug(&self, message: &str) {
        self.output(Level::Debug, message);
    }

    /// Output an info message.
    pub fn info(&self, message: &str) {
        self.output(Level::Info, message);
    }

    /// Output a success message.
    pub fn success(&self, message: &str) {
        self.output(Level::Success, message);
    }

    /// Output a warning message.
    pub fn warn(&self, message: &str) {
        self.output(Level::Warn, message);
    }

    /// Output an error message.
    pub fn error(&self, message: &str) {
        self.output(Level::Error, message);
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::from_env()
    }
}

// Global console access

/// Initialize the global console with a configuration.
/// This should be called early in main() if you need custom config.
/// If not called, `console()` will auto-initialize from environment.
pub fn init(config: ConsoleConfig) {
    let _ = CONSOLE.set(Console::new(config));
    INITIALIZED.store(true, Ordering::SeqCst);
}

/// Get the global console instance.
/// Auto-initializes from environment if not already initialized.
pub fn console() -> &'static Console {
    CONSOLE.get_or_init(|| {
        INITIALIZED.store(true, Ordering::SeqCst);
        Console::from_env()
    })
}

/// Check if the global console is initialized.
pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::SeqCst)
}

// Global convenience functions

/// Output a trace message to the global console.
pub fn trace(message: &str) {
    console().trace(message);
}

/// Output a debug message to the global console.
pub fn debug(message: &str) {
    console().debug(message);
}

/// Output an info message to the global console.
pub fn info(message: &str) {
    console().info(message);
}

/// Output a success message to the global console.
pub fn success(message: &str) {
    console().success(message);
}

/// Output a warning message to the global console.
pub fn warn(message: &str) {
    console().warn(message);
}

/// Output an error message to the global console.
pub fn error(message: &str) {
    console().error(message);
}

/// Output structured data to the global console.
pub fn data<T: Serialize>(data: &T) {
    console().output_data(data);
}

/// Output a structured message to the global console.
pub fn message(msg: &OutputMessage) {
    console().output_message(msg);
}
