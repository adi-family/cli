// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Yes/No confirmation input component.

use super::types::generate_id;
use crate::{console as out_console, is_interactive, theme, OutputMode};
use chrono::Utc;
use console::{Key, Term};
use std::io::Write;

/// Confirmation prompt builder.
pub struct Confirm {
    prompt: String,
    default: Option<bool>,
}

impl Confirm {
    /// Create a new confirm prompt.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            default: None,
        }
    }

    /// Set the default value (shown when Enter is pressed).
    pub fn default(mut self, value: bool) -> Self {
        self.default = Some(value);
        self
    }

    /// Run the confirmation and return the result.
    pub fn run(self) -> Option<bool> {
        let mode = out_console().mode();
        let interactive = is_interactive() && mode.is_text();

        match mode {
            OutputMode::JsonStream => self.run_json(),
            OutputMode::Text if interactive => self.run_interactive(),
            OutputMode::Text => self.run_simple(),
        }
    }

    /// Interactive mode with y/n keys.
    fn run_interactive(self) -> Option<bool> {
        let term = Term::stdout();

        let hint = match self.default {
            Some(true) => "[Y/n]",
            Some(false) => "[y/N]",
            None => "[y/n]",
        };

        print!("{} {} ", self.prompt, theme::muted(hint));
        let _ = std::io::stdout().flush();

        loop {
            match term.read_key() {
                Ok(Key::Char('y' | 'Y')) => {
                    println!("{}", theme::success("yes"));
                    return Some(true);
                }
                Ok(Key::Char('n' | 'N')) => {
                    println!("{}", theme::error("no"));
                    return Some(false);
                }
                Ok(Key::Enter) => {
                    if let Some(default) = self.default {
                        if default {
                            println!("{}", theme::success("yes"));
                        } else {
                            println!("{}", theme::error("no"));
                        }
                        return Some(default);
                    }
                }
                Ok(Key::Escape) => {
                    println!("{}", theme::muted("cancelled"));
                    return None;
                }
                _ => {}
            }
        }
    }

    /// Simple mode - read line.
    fn run_simple(self) -> Option<bool> {
        let hint = match self.default {
            Some(true) => "[Y/n]",
            Some(false) => "[y/N]",
            None => "[y/n]",
        };

        print!("{} {} ", self.prompt, hint);
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return None;
        }

        let input = input.trim().to_lowercase();

        if input.is_empty() {
            return self.default;
        }

        match input.as_str() {
            "y" | "yes" | "true" | "1" => Some(true),
            "n" | "no" | "false" | "0" => Some(false),
            _ => self.default,
        }
    }

    /// JSON mode.
    fn run_json(self) -> Option<bool> {
        let id = generate_id();
        let request = super::types::InputRequest::Confirm {
            id: id.clone(),
            prompt: self.prompt,
            default: self.default,
            timestamp: Utc::now(),
        };

        println!("{}", request.to_json());

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return None;
        }

        if let Some(event) = super::types::InputEvent::from_json(input.trim()) {
            match event {
                super::types::InputEvent::ConfirmResponse { value, .. } => return Some(value),
                super::types::InputEvent::Cancelled { .. } => return None,
                _ => {}
            }
        }

        None
    }
}

/// Convenience function for quick confirmation.
pub fn confirm(prompt: impl Into<String>) -> Option<bool> {
    Confirm::new(prompt).run()
}
