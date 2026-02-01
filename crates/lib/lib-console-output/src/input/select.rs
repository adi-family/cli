// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Single selection input component.

use super::types::{generate_id, InputRequest, SelectOption, SelectOptionJson};
use crate::{console as out_console, is_interactive, OutputMode};
use chrono::Utc;
use console::{style, Key, Term};
use std::io::Write;

/// Single selection builder.
pub struct Select<T: Clone> {
    prompt: String,
    options: Vec<SelectOption<T>>,
    default: Option<usize>,
}

impl<T: Clone> Select<T> {
    /// Create a new select with a prompt.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            options: Vec::new(),
            default: None,
        }
    }

    /// Add an option.
    pub fn option(mut self, option: SelectOption<T>) -> Self {
        self.options.push(option);
        self
    }

    /// Add multiple options.
    pub fn options(mut self, options: impl IntoIterator<Item = SelectOption<T>>) -> Self {
        self.options.extend(options);
        self
    }

    /// Add options from simple label-value pairs.
    pub fn items(mut self, items: impl IntoIterator<Item = (impl Into<String>, T)>) -> Self {
        for (label, value) in items {
            self.options.push(SelectOption::new(label, value));
        }
        self
    }

    /// Set the default selection index.
    pub fn default(mut self, index: usize) -> Self {
        self.default = Some(index);
        self
    }

    /// Run the selection and return the chosen value.
    pub fn run(self) -> Option<T> {
        if self.options.is_empty() {
            return None;
        }

        let mode = out_console().mode();
        let interactive = is_interactive() && mode.is_text();

        match mode {
            OutputMode::JsonStream => self.run_json(),
            OutputMode::Text if interactive => self.run_interactive(),
            OutputMode::Text => self.run_simple(),
        }
    }

    /// Interactive mode with arrow keys.
    fn run_interactive(self) -> Option<T> {
        let term = Term::stdout();
        let mut cursor = self.default.unwrap_or(0);

        // Find first non-disabled option
        while cursor < self.options.len() && self.options[cursor].disabled {
            cursor += 1;
        }
        if cursor >= self.options.len() {
            cursor = 0;
            while cursor < self.options.len() && self.options[cursor].disabled {
                cursor += 1;
            }
        }

        // Print prompt
        println!("{}", style(&self.prompt).bold());

        // Initial render
        self.render(&term, cursor);

        loop {
            match term.read_key() {
                Ok(Key::ArrowUp | Key::Char('k')) => {
                    cursor = self.find_prev_enabled(cursor);
                    self.render(&term, cursor);
                }
                Ok(Key::ArrowDown | Key::Char('j')) => {
                    cursor = self.find_next_enabled(cursor);
                    self.render(&term, cursor);
                }
                Ok(Key::Enter) => {
                    if !self.options[cursor].disabled {
                        self.clear_options(&term);
                        println!(
                            "{} {}",
                            style("\u{2713}").green(),
                            self.options[cursor].label
                        );
                        return Some(self.options[cursor].value.clone());
                    }
                }
                Ok(Key::Escape | Key::Char('q')) => {
                    self.clear_options(&term);
                    println!("{} Cancelled", style("\u{2715}").red());
                    return None;
                }
                _ => {}
            }
        }
    }

    /// Render options list.
    fn render(&self, term: &Term, cursor: usize) {
        // Move up to clear previous render
        let _ = term.clear_last_lines(self.options.len());

        for (i, opt) in self.options.iter().enumerate() {
            let is_selected = i == cursor;
            let prefix = if is_selected { ">" } else { " " };

            let line = if opt.disabled {
                format!(
                    "{} {} {}",
                    style(prefix).dim(),
                    style(&opt.label).dim(),
                    style("(disabled)").dim()
                )
            } else if is_selected {
                format!("{} {}", style(prefix).cyan(), style(&opt.label).cyan())
            } else {
                format!("  {}", &opt.label)
            };

            println!("{}", line);
        }
    }

    /// Clear the options list.
    fn clear_options(&self, term: &Term) {
        let _ = term.clear_last_lines(self.options.len());
    }

    /// Find next enabled option.
    fn find_next_enabled(&self, current: usize) -> usize {
        let mut next = (current + 1) % self.options.len();
        let start = next;
        while self.options[next].disabled {
            next = (next + 1) % self.options.len();
            if next == start {
                return current; // All disabled
            }
        }
        next
    }

    /// Find previous enabled option.
    fn find_prev_enabled(&self, current: usize) -> usize {
        let mut prev = if current == 0 {
            self.options.len() - 1
        } else {
            current - 1
        };
        let start = prev;
        while self.options[prev].disabled {
            prev = if prev == 0 {
                self.options.len() - 1
            } else {
                prev - 1
            };
            if prev == start {
                return current; // All disabled
            }
        }
        prev
    }

    /// Simple mode - just number selection.
    fn run_simple(self) -> Option<T> {
        println!("{}", style(&self.prompt).bold());
        for (i, opt) in self.options.iter().enumerate() {
            if opt.disabled {
                println!(
                    "  {} {} (disabled)",
                    style(format!("[{}]", i + 1)).dim(),
                    style(&opt.label).dim()
                );
            } else {
                println!("  {} {}", style(format!("[{}]", i + 1)).cyan(), &opt.label);
            }
        }

        let default_hint = self
            .default
            .map(|d| format!(" [{}]", d + 1))
            .unwrap_or_default();
        print!("Enter number{}: ", default_hint);
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return None;
        }

        let input = input.trim();
        let index = if input.is_empty() {
            self.default
        } else {
            input.parse::<usize>().ok().map(|n| n.saturating_sub(1))
        };

        index.and_then(|i| {
            if i < self.options.len() && !self.options[i].disabled {
                Some(self.options[i].value.clone())
            } else {
                None
            }
        })
    }

    /// JSON mode - output request and wait for response.
    fn run_json(self) -> Option<T> {
        let id = generate_id();
        let request = InputRequest::Select {
            id: id.clone(),
            prompt: self.prompt,
            options: self.options.iter().map(SelectOptionJson::from).collect(),
            default: self.default,
            timestamp: Utc::now(),
        };

        println!("{}", request.to_json());

        // Read response from stdin
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return None;
        }

        // Parse response
        if let Some(event) = super::types::InputEvent::from_json(input.trim()) {
            match event {
                super::types::InputEvent::SelectResponse { index, .. } => {
                    if index < self.options.len() && !self.options[index].disabled {
                        return Some(self.options[index].value.clone());
                    }
                }
                super::types::InputEvent::Cancelled { .. } => return None,
                _ => {}
            }
        }

        None
    }
}

/// Convenience function for quick selection.
pub fn select<T: Clone>(
    prompt: impl Into<String>,
    options: impl IntoIterator<Item = SelectOption<T>>,
) -> Option<T> {
    Select::new(prompt).options(options).run()
}
