// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Text input component.

use super::types::generate_id;
use crate::{console as out_console, is_interactive, OutputMode};
use chrono::Utc;
use console::{style, Key, Term};
use std::io::Write;

/// Validator function type alias.
type ValidatorFn = Box<dyn Fn(&str) -> Result<(), String>>;

/// Text input builder.
pub struct Input {
    prompt: String,
    default: Option<String>,
    placeholder: Option<String>,
    validator: Option<ValidatorFn>,
    allow_empty: bool,
}

impl Input {
    /// Create a new text input.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            default: None,
            placeholder: None,
            validator: None,
            allow_empty: true,
        }
    }

    /// Set a default value.
    pub fn default(mut self, value: impl Into<String>) -> Self {
        self.default = Some(value.into());
        self
    }

    /// Set a placeholder hint.
    pub fn placeholder(mut self, value: impl Into<String>) -> Self {
        self.placeholder = Some(value.into());
        self
    }

    /// Set a validator function.
    pub fn validate<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) -> Result<(), String> + 'static,
    {
        self.validator = Some(Box::new(f));
        self
    }

    /// Require non-empty input.
    pub fn required(mut self) -> Self {
        self.allow_empty = false;
        self
    }

    /// Run the input and return the value.
    pub fn run(self) -> Option<String> {
        let mode = out_console().mode();
        let interactive = is_interactive() && mode.is_text();

        match mode {
            OutputMode::JsonStream => self.run_json(),
            OutputMode::Text if interactive => self.run_interactive(),
            OutputMode::Text => self.run_simple(),
        }
    }

    /// Interactive mode with line editing.
    fn run_interactive(self) -> Option<String> {
        let term = Term::stdout();
        let mut buffer = self.default.clone().unwrap_or_default();
        let mut cursor = buffer.len();

        loop {
            // Render prompt and current input
            let _ = term.clear_line();
            let default_hint = self
                .default
                .as_ref()
                .map(|d| format!(" [{}]", style(d).dim()))
                .unwrap_or_default();

            print!("\r{}{}: {}", self.prompt, default_hint, buffer);
            let _ = std::io::stdout().flush();

            match term.read_key() {
                Ok(Key::Enter) => {
                    let value = if buffer.is_empty() {
                        self.default.clone().unwrap_or_default()
                    } else {
                        buffer.clone()
                    };

                    // Validate
                    if !self.allow_empty && value.is_empty() {
                        println!();
                        println!("{} Input is required", style("!").yellow());
                        continue;
                    }

                    if let Some(ref validator) = self.validator {
                        if let Err(msg) = validator(&value) {
                            println!();
                            println!("{} {}", style("!").yellow(), msg);
                            continue;
                        }
                    }

                    println!();
                    return Some(value);
                }
                Ok(Key::Escape) => {
                    println!();
                    return None;
                }
                Ok(Key::Backspace) => {
                    if cursor > 0 {
                        cursor -= 1;
                        buffer.remove(cursor);
                    }
                }
                Ok(Key::Del) => {
                    if cursor < buffer.len() {
                        buffer.remove(cursor);
                    }
                }
                Ok(Key::ArrowLeft) => {
                    cursor = cursor.saturating_sub(1);
                }
                Ok(Key::ArrowRight) => {
                    if cursor < buffer.len() {
                        cursor += 1;
                    }
                }
                Ok(Key::Home) => {
                    cursor = 0;
                }
                Ok(Key::End) => {
                    cursor = buffer.len();
                }
                Ok(Key::Char(c)) => {
                    buffer.insert(cursor, c);
                    cursor += 1;
                }
                _ => {}
            }
        }
    }

    /// Simple mode - just read line.
    fn run_simple(self) -> Option<String> {
        let default_hint = self
            .default
            .as_ref()
            .map(|d| format!(" [{}]", d))
            .unwrap_or_default();

        print!("{}{}: ", self.prompt, default_hint);
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return None;
        }

        let value = input.trim();
        let value = if value.is_empty() {
            self.default.clone().unwrap_or_default()
        } else {
            value.to_string()
        };

        // Validate
        if !self.allow_empty && value.is_empty() {
            println!("{} Input is required", style("!").yellow());
            return None;
        }

        if let Some(ref validator) = self.validator {
            if let Err(msg) = validator(&value) {
                println!("{} {}", style("!").yellow(), msg);
                return None;
            }
        }

        Some(value)
    }

    /// JSON mode.
    fn run_json(self) -> Option<String> {
        let id = generate_id();
        let request = super::types::InputRequest::Input {
            id: id.clone(),
            prompt: self.prompt,
            default: self.default,
            placeholder: self.placeholder,
            timestamp: Utc::now(),
        };

        println!("{}", request.to_json());

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return None;
        }

        if let Some(event) = super::types::InputEvent::from_json(input.trim()) {
            match event {
                super::types::InputEvent::InputResponse { value, .. } => {
                    // Validate
                    if !self.allow_empty && value.is_empty() {
                        return None;
                    }
                    if let Some(ref validator) = self.validator {
                        if validator(&value).is_err() {
                            return None;
                        }
                    }
                    return Some(value);
                }
                super::types::InputEvent::Cancelled { .. } => return None,
                _ => {}
            }
        }

        None
    }
}

/// Convenience function for quick text input.
pub fn text_input(prompt: impl Into<String>) -> Option<String> {
    Input::new(prompt).run()
}
