// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Single selection input component.

use super::types::{generate_id, InputRequest, SelectOption, SelectOptionJson};
use crate::{console as out_console, fg_println, is_interactive, theme, OutputMode};
use chrono::Utc;
use console::{Key, Term};
use std::io::Write;

/// Single selection builder.
pub struct Select<T: Clone> {
    prompt: String,
    options: Vec<SelectOption<T>>,
    default: Option<usize>,
    /// Enable filtering/autocomplete mode
    filterable: bool,
    /// Maximum number of options to display in filterable mode (default: all)
    /// When there are more matches, shows "and N more" message
    max_display: Option<usize>,
}

impl<T: Clone> Select<T> {
    /// Create a new select with a prompt.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            options: Vec::new(),
            default: None,
            filterable: false,
            max_display: None,
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

    /// Enable filtering/autocomplete mode.
    /// When enabled, users can type to filter options.
    pub fn filterable(mut self, enabled: bool) -> Self {
        self.filterable = enabled;
        self
    }

    /// Set maximum number of options to display in filterable mode.
    /// When there are more matches than this limit, shows "and N more" message.
    /// Only applies when filterable mode is enabled.
    pub fn max_display(mut self, count: Option<usize>) -> Self {
        self.max_display = count;
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
            OutputMode::Text if interactive && self.filterable => self.run_filterable(),
            OutputMode::Text if interactive => self.run_interactive(),
            OutputMode::Text => self.run_simple(),
        }
    }

    /// Interactive mode with arrow keys.
    fn run_interactive(self) -> Option<T> {
        let term = Term::stdout();
        let mut cursor = self.default.unwrap_or(0);
        let mut rendered_once = false;

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
        fg_println!("{}", theme::bold(&self.prompt));

        // Initial render (don't clear on first render)
        self.render(&term, cursor, rendered_once);
        rendered_once = true;

        loop {
            match term.read_key() {
                Ok(Key::ArrowUp | Key::Char('k')) => {
                    cursor = self.find_prev_enabled(cursor);
                    self.render(&term, cursor, rendered_once);
                }
                Ok(Key::ArrowDown | Key::Char('j')) => {
                    cursor = self.find_next_enabled(cursor);
                    self.render(&term, cursor, rendered_once);
                }
                Ok(Key::Enter) => {
                    if !self.options[cursor].disabled {
                        self.clear_options(&term);
                        fg_println!(
                            "{} {}",
                            theme::success(theme::icons::SUCCESS),
                            theme::foreground(&self.options[cursor].label)
                        );
                        return Some(self.options[cursor].value.clone());
                    }
                }
                Ok(Key::Escape | Key::Char('q')) => {
                    self.clear_options(&term);
                    fg_println!("{} {}", theme::error(theme::icons::ERROR), theme::foreground("Cancelled"));
                    return None;
                }
                _ => {}
            }
        }
    }

    /// Filterable mode with typing to filter options.
    fn run_filterable(self) -> Option<T> {
        let term = Term::stdout();
        let mut filter = String::new();
        let mut cursor: usize = 0;
        let mut prev_lines: usize = 0; // Track actual lines from previous render

        // Print prompt with filter hint
        fg_println!(
            "{} {}",
            theme::bold(&self.prompt),
            theme::muted("(type to filter)")
        );

        // Initial render (don't clear on first render since prev_lines = 0)
        let filtered = self.filter_options(&filter);
        prev_lines = self.render_filterable(&term, &filter, &filtered, cursor, prev_lines);

        loop {
            match term.read_key() {
                Ok(Key::ArrowUp | Key::Char('k')) if filter.is_empty() => {
                    let filtered = self.filter_options(&filter);
                    if !filtered.is_empty() {
                        cursor = if cursor == 0 {
                            filtered.len() - 1
                        } else {
                            cursor - 1
                        };
                        // Skip disabled options
                        let start = cursor;
                        while self.options[filtered[cursor]].disabled {
                            cursor = if cursor == 0 {
                                filtered.len() - 1
                            } else {
                                cursor - 1
                            };
                            if cursor == start {
                                break;
                            }
                        }
                    }
                    prev_lines =
                        self.render_filterable(&term, &filter, &filtered, cursor, prev_lines);
                }
                Ok(Key::ArrowDown | Key::Char('j')) if filter.is_empty() => {
                    let filtered = self.filter_options(&filter);
                    if !filtered.is_empty() {
                        cursor = (cursor + 1) % filtered.len();
                        // Skip disabled options
                        let start = cursor;
                        while self.options[filtered[cursor]].disabled {
                            cursor = (cursor + 1) % filtered.len();
                            if cursor == start {
                                break;
                            }
                        }
                    }
                    prev_lines =
                        self.render_filterable(&term, &filter, &filtered, cursor, prev_lines);
                }
                Ok(Key::ArrowUp) => {
                    let filtered = self.filter_options(&filter);
                    if !filtered.is_empty() {
                        cursor = if cursor == 0 {
                            filtered.len() - 1
                        } else {
                            cursor - 1
                        };
                        // Skip disabled options
                        let start = cursor;
                        while self.options[filtered[cursor]].disabled {
                            cursor = if cursor == 0 {
                                filtered.len() - 1
                            } else {
                                cursor - 1
                            };
                            if cursor == start {
                                break;
                            }
                        }
                    }
                    prev_lines =
                        self.render_filterable(&term, &filter, &filtered, cursor, prev_lines);
                }
                Ok(Key::ArrowDown) => {
                    let filtered = self.filter_options(&filter);
                    if !filtered.is_empty() {
                        cursor = (cursor + 1) % filtered.len();
                        // Skip disabled options
                        let start = cursor;
                        while self.options[filtered[cursor]].disabled {
                            cursor = (cursor + 1) % filtered.len();
                            if cursor == start {
                                break;
                            }
                        }
                    }
                    prev_lines =
                        self.render_filterable(&term, &filter, &filtered, cursor, prev_lines);
                }
                Ok(Key::Enter) => {
                    let filtered = self.filter_options(&filter);
                    if !filtered.is_empty() {
                        let original_idx = filtered[cursor];
                        if !self.options[original_idx].disabled {
                            self.clear_filterable(&term, prev_lines);
                            fg_println!(
                                "{} {}",
                                theme::success(theme::icons::SUCCESS),
                                theme::foreground(&self.options[original_idx].label)
                            );
                            return Some(self.options[original_idx].value.clone());
                        }
                    }
                }
                Ok(Key::Escape) => {
                    self.clear_filterable(&term, prev_lines);
                    fg_println!("{} {}", theme::error(theme::icons::ERROR), theme::foreground("Cancelled"));
                    return None;
                }
                Ok(Key::Backspace) => {
                    filter.pop();
                    cursor = 0;
                    let filtered = self.filter_options(&filter);
                    prev_lines =
                        self.render_filterable(&term, &filter, &filtered, cursor, prev_lines);
                }
                Ok(Key::Char(c)) if !c.is_control() => {
                    filter.push(c);
                    cursor = 0;
                    let filtered = self.filter_options(&filter);
                    prev_lines =
                        self.render_filterable(&term, &filter, &filtered, cursor, prev_lines);
                }
                _ => {}
            }
        }
    }

    /// Filter options based on search string (case-insensitive).
    /// Matches against both label and description.
    fn filter_options(&self, filter: &str) -> Vec<usize> {
        if filter.is_empty() {
            return (0..self.options.len()).collect();
        }

        let filter_lower = filter.to_lowercase();
        self.options
            .iter()
            .enumerate()
            .filter(|(_, opt)| {
                opt.label.to_lowercase().contains(&filter_lower)
                    || opt
                        .description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(&filter_lower))
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Compute the max label width for a set of option indices (for column alignment).
    fn max_label_width(&self, indices: &[usize]) -> usize {
        indices
            .iter()
            .map(|&i| self.options[i].label.len())
            .max()
            .unwrap_or(0)
    }

    /// Format description suffix with padding for column alignment.
    fn format_description(&self, opt: &SelectOption<T>, pad_to: usize) -> String {
        match &opt.description {
            Some(desc) if !desc.is_empty() => {
                let padding = pad_to.saturating_sub(opt.label.len());
                format!(
                    "{}  {}",
                    " ".repeat(padding),
                    theme::muted(desc)
                )
            }
            _ => String::new(),
        }
    }

    /// Render filterable options list with filter input.
    /// Returns the number of lines rendered (for clearing on next render).
    fn render_filterable(
        &self,
        term: &Term,
        filter: &str,
        filtered: &[usize],
        cursor: usize,
        prev_lines: usize,
    ) -> usize {
        // Clear previous render (only if we've rendered before)
        if prev_lines > 0 {
            let _ = term.clear_last_lines(prev_lines);
        }

        // Calculate how many options to display and the sliding window
        let max_visible = self.max_display.unwrap_or(filtered.len());
        let display_count = max_visible.min(filtered.len());

        // Calculate window start for sliding window (keep cursor visible)
        let window_start = if filtered.is_empty() {
            0
        } else if cursor >= display_count {
            // Cursor is beyond visible area, slide window
            cursor.saturating_sub(display_count - 1)
        } else {
            0
        };

        let window_end = (window_start + display_count).min(filtered.len());
        let has_more_above = window_start > 0;
        let has_more_below = window_end < filtered.len();

        // Render filter input line
        if filter.is_empty() {
            fg_println!("{}", theme::muted("  Type to filter..."));
        } else {
            fg_println!(
                "  {} {} {}",
                theme::muted("Filter:"),
                theme::foreground(filter),
                theme::muted(format!("({} matches)", filtered.len()))
            );
        }

        let mut lines_rendered = 1; // filter line

        // Render filtered options with sliding window
        if filtered.is_empty() {
            fg_println!("{}", theme::error("  No matches"));
            lines_rendered += 1;
        } else {
            // Show "N more above" if there are hidden options above
            if has_more_above {
                fg_println!(
                    "{}",
                    theme::muted(format!("  ... {} more above", window_start))
                );
                lines_rendered += 1;
            }

            // Compute max label width for visible window for alignment
            let visible_indices: Vec<usize> = (window_start..window_end)
                .map(|i| filtered[i])
                .collect();
            let max_width = self.max_label_width(&visible_indices);

            for i in window_start..window_end {
                let original_idx = filtered[i];
                let opt = &self.options[original_idx];
                let is_selected = i == cursor;
                let prefix = if is_selected { ">" } else { " " };
                let desc = self.format_description(opt, max_width);

                let line = if opt.disabled {
                    format!(
                        "{} {} {}",
                        theme::muted(prefix),
                        theme::muted(&opt.label),
                        theme::muted("(disabled)")
                    )
                } else if is_selected {
                    format!("{} {}{}", theme::brand(prefix), theme::brand(&opt.label), desc)
                } else {
                    format!("  {}{}", theme::foreground(&opt.label), desc)
                };

                fg_println!("{}", line);
                lines_rendered += 1;
            }

            // Show "and N more" if there are hidden options below
            if has_more_below {
                let more_count = filtered.len() - window_end;
                fg_println!("{}", theme::muted(format!("  ... and {} more", more_count)));
                lines_rendered += 1;
            }
        }

        lines_rendered
    }

    /// Clear the filterable options list.
    fn clear_filterable(&self, term: &Term, prev_lines: usize) {
        if prev_lines > 0 {
            let _ = term.clear_last_lines(prev_lines);
        }
    }

    /// Render options list.
    fn render(&self, term: &Term, cursor: usize, should_clear: bool) {
        // Clear previous render only if we've rendered before
        if should_clear {
            let _ = term.clear_last_lines(self.options.len());
        }

        let all_indices: Vec<usize> = (0..self.options.len()).collect();
        let max_width = self.max_label_width(&all_indices);

        for (i, opt) in self.options.iter().enumerate() {
            let is_selected = i == cursor;
            let prefix = if is_selected { ">" } else { " " };
            let desc = self.format_description(opt, max_width);

            let line = if opt.disabled {
                format!(
                    "{} {} {}",
                    theme::muted(prefix),
                    theme::muted(&opt.label),
                    theme::muted("(disabled)")
                )
            } else if is_selected {
                format!("{} {}{}", theme::brand(prefix), theme::brand(&opt.label), desc)
            } else {
                format!("  {}{}", theme::foreground(&opt.label), desc)
            };

            fg_println!("{}", line);
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
        let all_indices: Vec<usize> = (0..self.options.len()).collect();
        let max_width = self.max_label_width(&all_indices);

        fg_println!("{}", theme::bold(&self.prompt));
        for (i, opt) in self.options.iter().enumerate() {
            let desc = self.format_description(opt, max_width);
            if opt.disabled {
                fg_println!(
                    "  {} {} (disabled)",
                    theme::muted(format!("[{}]", i + 1)),
                    theme::muted(&opt.label)
                );
            } else {
                fg_println!("  {} {}{}", theme::brand(format!("[{}]", i + 1)), theme::foreground(&opt.label), desc);
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
