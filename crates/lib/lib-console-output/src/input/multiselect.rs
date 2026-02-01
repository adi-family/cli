// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Multiple selection input component.

use super::types::{generate_id, InputRequest, SelectOption, SelectOptionJson};
use crate::{console as out_console, is_interactive, OutputMode};
use chrono::Utc;
use console::{style, Key, Term};
use std::collections::HashSet;
use std::io::Write;

/// Multiple selection builder.
pub struct MultiSelect<T: Clone> {
    prompt: String,
    options: Vec<SelectOption<T>>,
    defaults: HashSet<usize>,
    min: Option<usize>,
    max: Option<usize>,
}

impl<T: Clone> MultiSelect<T> {
    /// Create a new multiselect with a prompt.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            options: Vec::new(),
            defaults: HashSet::new(),
            min: None,
            max: None,
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

    /// Set default selected indices.
    pub fn defaults(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.defaults = indices.into_iter().collect();
        self
    }

    /// Set minimum required selections.
    pub fn min(mut self, min: usize) -> Self {
        self.min = Some(min);
        self
    }

    /// Set maximum allowed selections.
    pub fn max(mut self, max: usize) -> Self {
        self.max = Some(max);
        self
    }

    /// Run the selection and return chosen values.
    pub fn run(self) -> Option<Vec<T>> {
        if self.options.is_empty() {
            return Some(Vec::new());
        }

        let mode = out_console().mode();
        let interactive = is_interactive() && mode.is_text();

        match mode {
            OutputMode::JsonStream => self.run_json(),
            OutputMode::Text if interactive => self.run_interactive(),
            OutputMode::Text => self.run_simple(),
        }
    }

    /// Interactive mode with arrow keys and space to toggle.
    fn run_interactive(self) -> Option<Vec<T>> {
        let term = Term::stdout();
        let mut cursor = 0;
        let mut selected: HashSet<usize> = self.defaults.clone();

        // Find first non-disabled option
        while cursor < self.options.len() && self.options[cursor].disabled {
            cursor += 1;
        }

        // Print prompt and instructions
        println!("{}", style(&self.prompt).bold());
        println!(
            "{}",
            style("(Space to toggle, Enter to confirm, Esc to cancel)").dim()
        );

        // Initial render
        self.render(&term, cursor, &selected);

        loop {
            match term.read_key() {
                Ok(Key::ArrowUp | Key::Char('k')) => {
                    cursor = self.find_prev_enabled(cursor);
                    self.render(&term, cursor, &selected);
                }
                Ok(Key::ArrowDown | Key::Char('j')) => {
                    cursor = self.find_next_enabled(cursor);
                    self.render(&term, cursor, &selected);
                }
                Ok(Key::Char(' ')) => {
                    if !self.options[cursor].disabled {
                        if selected.contains(&cursor) {
                            selected.remove(&cursor);
                        } else if self.max.is_none() || selected.len() < self.max.unwrap() {
                            selected.insert(cursor);
                        }
                        self.render(&term, cursor, &selected);
                    }
                }
                Ok(Key::Char('a')) => {
                    // Toggle all
                    if selected.len() == self.options.iter().filter(|o| !o.disabled).count() {
                        selected.clear();
                    } else {
                        for (i, opt) in self.options.iter().enumerate() {
                            if !opt.disabled
                                && (self.max.is_none() || selected.len() < self.max.unwrap())
                            {
                                selected.insert(i);
                            }
                        }
                    }
                    self.render(&term, cursor, &selected);
                }
                Ok(Key::Enter) => {
                    if let Some(min) = self.min {
                        if selected.len() < min {
                            continue; // Not enough selections
                        }
                    }
                    self.clear_options(&term);
                    let values: Vec<T> = selected
                        .iter()
                        .filter_map(|&i| self.options.get(i).map(|o| o.value.clone()))
                        .collect();
                    let labels: Vec<&str> = selected
                        .iter()
                        .filter_map(|&i| self.options.get(i).map(|o| o.label.as_str()))
                        .collect();
                    println!(
                        "{} {}",
                        style("\u{2713}").green(),
                        if labels.is_empty() {
                            "(none)".to_string()
                        } else {
                            labels.join(", ")
                        }
                    );
                    return Some(values);
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

    /// Render options list with checkboxes.
    fn render(&self, term: &Term, cursor: usize, selected: &HashSet<usize>) {
        let _ = term.clear_last_lines(self.options.len());

        for (i, opt) in self.options.iter().enumerate() {
            let is_cursor = i == cursor;
            let is_selected = selected.contains(&i);
            let prefix = if is_cursor { ">" } else { " " };
            let checkbox = if is_selected { "[\u{2713}]" } else { "[ ]" };

            let line = if opt.disabled {
                format!(
                    "{} {} {} {}",
                    style(prefix).dim(),
                    style(checkbox).dim(),
                    style(&opt.label).dim(),
                    style("(disabled)").dim()
                )
            } else if is_cursor {
                format!(
                    "{} {} {}",
                    style(prefix).cyan(),
                    if is_selected {
                        style(checkbox).green()
                    } else {
                        style(checkbox).cyan()
                    },
                    style(&opt.label).cyan()
                )
            } else {
                format!(
                    "  {} {}",
                    if is_selected {
                        style(checkbox).green()
                    } else {
                        style(checkbox).white()
                    },
                    &opt.label
                )
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
                return current;
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
                return current;
            }
        }
        prev
    }

    /// Simple mode - comma-separated number selection.
    fn run_simple(self) -> Option<Vec<T>> {
        println!("{}", style(&self.prompt).bold());
        for (i, opt) in self.options.iter().enumerate() {
            let marker = if self.defaults.contains(&i) { "*" } else { " " };
            if opt.disabled {
                println!(
                    " {} {} {} (disabled)",
                    marker,
                    style(format!("[{}]", i + 1)).dim(),
                    style(&opt.label).dim()
                );
            } else {
                println!(
                    " {} {} {}",
                    marker,
                    style(format!("[{}]", i + 1)).cyan(),
                    &opt.label
                );
            }
        }

        let defaults_str = if self.defaults.is_empty() {
            String::new()
        } else {
            format!(
                " [{}]",
                self.defaults
                    .iter()
                    .map(|i| (i + 1).to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        print!("Enter numbers (comma-separated){}: ", defaults_str);
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return None;
        }

        let input = input.trim();
        let indices: HashSet<usize> = if input.is_empty() {
            self.defaults.clone()
        } else {
            input
                .split(',')
                .filter_map(|s| s.trim().parse::<usize>().ok().map(|n| n.saturating_sub(1)))
                .collect()
        };

        // Validate
        if let Some(min) = self.min {
            if indices.len() < min {
                println!(
                    "{} At least {} selection(s) required",
                    style("!").yellow(),
                    min
                );
                return None;
            }
        }

        let values: Vec<T> = indices
            .iter()
            .filter_map(|&i| {
                if i < self.options.len() && !self.options[i].disabled {
                    Some(self.options[i].value.clone())
                } else {
                    None
                }
            })
            .collect();

        Some(values)
    }

    /// JSON mode.
    fn run_json(self) -> Option<Vec<T>> {
        let id = generate_id();
        let request = InputRequest::MultiSelect {
            id: id.clone(),
            prompt: self.prompt,
            options: self.options.iter().map(SelectOptionJson::from).collect(),
            defaults: self.defaults.iter().cloned().collect(),
            min: self.min,
            max: self.max,
            timestamp: Utc::now(),
        };

        println!("{}", request.to_json());

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() {
            return None;
        }

        if let Some(event) = super::types::InputEvent::from_json(input.trim()) {
            match event {
                super::types::InputEvent::MultiSelectResponse { indices, .. } => {
                    let values: Vec<T> = indices
                        .iter()
                        .filter_map(|&i| {
                            if i < self.options.len() && !self.options[i].disabled {
                                Some(self.options[i].value.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    return Some(values);
                }
                super::types::InputEvent::Cancelled { .. } => return None,
                _ => {}
            }
        }

        None
    }
}

/// Convenience function for quick multi-selection.
pub fn multi_select<T: Clone>(
    prompt: impl Into<String>,
    options: impl IntoIterator<Item = SelectOption<T>>,
) -> Option<Vec<T>> {
    MultiSelect::new(prompt).options(options).run()
}
