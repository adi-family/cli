// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Key-value pair display component.
//!
//! ```text
//!   PID              1234
//!   Version          0.5.0
//!   Uptime           3600s
//!   Running services 4/5
//! ```

use super::{visible_width, LiveHandle, Renderable};
use crate::theme;
use console::Term;
use std::fmt;
use std::io::Write;

/// Static key-value pair display.
pub struct KeyValue {
    entries: Vec<(String, String)>,
    separator: String,
    indent: usize,
}

impl KeyValue {
    /// Create a new empty key-value display.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            separator: "  ".to_string(),
            indent: 2,
        }
    }

    /// Add a key-value entry.
    pub fn entry(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.entries.push((key.into(), value.into()));
        self
    }

    /// Set the separator between key and value (default: 2 spaces).
    pub fn separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }

    /// Set indentation (number of leading spaces, default: 2).
    pub fn indent(mut self, spaces: usize) -> Self {
        self.indent = spaces;
        self
    }

    /// Get the max visible key width for alignment.
    fn max_key_width(&self) -> usize {
        self.entries
            .iter()
            .map(|(k, _)| visible_width(k))
            .max()
            .unwrap_or(0)
    }
}

impl Default for KeyValue {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for KeyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let max_key = self.max_key_width();
        let pad = " ".repeat(self.indent);

        for (key, value) in &self.entries {
            let key_visible = visible_width(key);
            let key_padding = " ".repeat(max_key.saturating_sub(key_visible));
            writeln!(
                f,
                "{}{}{}{}{}",
                pad,
                theme::muted(key),
                key_padding,
                self.separator,
                value
            )?;
        }
        Ok(())
    }
}

impl Renderable for KeyValue {
    fn line_count(&self) -> usize {
        self.entries.len()
    }
}

/// Live key-value display that auto-refreshes on mutation.
pub struct LiveKeyValue {
    entries: Vec<(String, String)>,
    separator: String,
    indent: usize,
    rendered_lines: usize,
    term: Term,
}

impl LiveKeyValue {
    /// Create a new live key-value display.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            separator: "  ".to_string(),
            indent: 2,
            rendered_lines: 0,
            term: Term::stdout(),
        }
    }

    /// Set the separator between key and value (default: 2 spaces).
    pub fn separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }

    /// Set indentation (number of leading spaces, default: 2).
    pub fn indent(mut self, spaces: usize) -> Self {
        self.indent = spaces;
        self
    }

    /// Set or update a key-value entry. If key exists, updates value and re-renders.
    /// If key is new, appends and re-renders.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();

        let existing = self.entries.iter().position(|(k, _)| *k == key);
        match existing {
            Some(idx) => {
                self.entries[idx].1 = value;
                self.rerender();
            }
            None => {
                self.entries.push((key, value));
                self.rerender();
            }
        }
    }

    /// Finalize the display — output stays, no more clearing.
    pub fn done(self) -> LiveHandle {
        LiveHandle::new(self.rendered_lines)
    }

    /// Build the static KeyValue for rendering.
    fn to_static(&self) -> KeyValue {
        let mut kv = KeyValue::new()
            .separator(self.separator.clone())
            .indent(self.indent);
        kv.entries = self.entries.clone();
        kv
    }

    /// Clear and re-render all entries.
    fn rerender(&mut self) {
        if self.rendered_lines > 0 {
            let _ = self.term.clear_last_lines(self.rendered_lines);
        }

        let static_kv = self.to_static();
        let rendered = static_kv.render();
        self.rendered_lines = static_kv.line_count();

        let _ = write!(&self.term, "{}", rendered);
        let _ = self.term.flush();
    }
}

impl Default for LiveKeyValue {
    fn default() -> Self {
        Self::new()
    }
}
