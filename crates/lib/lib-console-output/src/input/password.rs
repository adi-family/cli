// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Password (hidden) input component.

use super::types::generate_id;
use crate::{console as out_console, is_interactive, OutputMode};
use chrono::Utc;
use console::{style, Key, Term};
use std::io::Write;

/// Password input builder.
pub struct Password {
    prompt: String,
    confirmation: Option<String>,
    mask: char,
    allow_empty: bool,
}

impl Password {
    /// Create a new password input.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            confirmation: None,
            mask: '*',
            allow_empty: false,
        }
    }

    /// Require password confirmation (re-enter).
    pub fn confirm(mut self, confirmation_prompt: impl Into<String>) -> Self {
        self.confirmation = Some(confirmation_prompt.into());
        self
    }

    /// Set the mask character (default: '*').
    pub fn mask(mut self, c: char) -> Self {
        self.mask = c;
        self
    }

    /// Allow empty password.
    pub fn allow_empty(mut self) -> Self {
        self.allow_empty = true;
        self
    }

    /// Run the password input and return the value.
    pub fn run(self) -> Option<String> {
        let mode = out_console().mode();
        let interactive = is_interactive() && mode.is_text();

        match mode {
            OutputMode::JsonStream => self.run_json(),
            OutputMode::Text if interactive => self.run_interactive(),
            OutputMode::Text => self.run_simple(),
        }
    }

    /// Interactive mode with hidden input.
    fn run_interactive(self) -> Option<String> {
        let term = Term::stdout();

        loop {
            let password = self.read_password(&term, &self.prompt)?;

            // Check empty
            if !self.allow_empty && password.is_empty() {
                println!("{} Password cannot be empty", style("!").yellow());
                continue;
            }

            // Confirmation if required
            if let Some(ref confirm_prompt) = self.confirmation {
                let confirm = self.read_password(&term, confirm_prompt)?;
                if password != confirm {
                    println!("{} Passwords do not match", style("!").yellow());
                    continue;
                }
            }

            return Some(password);
        }
    }

    /// Read a single password.
    fn read_password(&self, term: &Term, prompt: &str) -> Option<String> {
        let mut buffer = String::new();

        print!("{}: ", prompt);
        let _ = std::io::stdout().flush();

        loop {
            match term.read_key() {
                Ok(Key::Enter) => {
                    println!();
                    return Some(buffer);
                }
                Ok(Key::Escape) => {
                    println!();
                    return None;
                }
                Ok(Key::Backspace) => {
                    if !buffer.is_empty() {
                        buffer.pop();
                        // Clear and reprint mask
                        print!(
                            "\r{}: {}",
                            prompt,
                            self.mask.to_string().repeat(buffer.len())
                        );
                        // Clear any extra characters
                        print!(" \x08");
                        let _ = std::io::stdout().flush();
                    }
                }
                Ok(Key::Char(c)) => {
                    buffer.push(c);
                    print!("{}", self.mask);
                    let _ = std::io::stdout().flush();
                }
                _ => {}
            }
        }
    }

    /// Simple mode - use terminal's built-in password reading.
    fn run_simple(self) -> Option<String> {
        let term = Term::stdout();

        loop {
            print!("{}: ", self.prompt);
            let _ = std::io::stdout().flush();

            let password = match term.read_secure_line() {
                Ok(p) => p,
                Err(_) => return None,
            };

            // Check empty
            if !self.allow_empty && password.is_empty() {
                println!("{} Password cannot be empty", style("!").yellow());
                continue;
            }

            // Confirmation if required
            if let Some(ref confirm_prompt) = self.confirmation {
                print!("{}: ", confirm_prompt);
                let _ = std::io::stdout().flush();

                let confirm = match term.read_secure_line() {
                    Ok(p) => p,
                    Err(_) => return None,
                };

                if password != confirm {
                    println!("{} Passwords do not match", style("!").yellow());
                    continue;
                }
            }

            return Some(password);
        }
    }

    /// JSON mode.
    fn run_json(self) -> Option<String> {
        let id = generate_id();
        let request = super::types::InputRequest::Password {
            id: id.clone(),
            prompt: self.prompt.clone(),
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
                    if !self.allow_empty && value.is_empty() {
                        return None;
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

/// Convenience function for quick password input.
pub fn password(prompt: impl Into<String>) -> Option<String> {
    Password::new(prompt).run()
}
