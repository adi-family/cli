// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Bullet and numbered list component.
//!
//! ```text
//!   • First item
//!   • Second item
//!   • Third item
//! ```

use super::Renderable;
use crate::theme;
use std::fmt;

/// Bullet or numbered list.
pub struct List {
    items: Vec<String>,
    numbered: bool,
    indent: usize,
}

impl List {
    /// Create a new empty list.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            numbered: false,
            indent: 2,
        }
    }

    /// Add an item to the list.
    pub fn item(mut self, text: impl Into<String>) -> Self {
        self.items.push(text.into());
        self
    }

    /// Add multiple items at once.
    pub fn items<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.items.extend(items.into_iter().map(Into::into));
        self
    }

    /// Enable numbered mode (1. 2. 3.) instead of bullets.
    pub fn numbered(mut self, enabled: bool) -> Self {
        self.numbered = enabled;
        self
    }

    /// Set indentation level (number of leading spaces, default: 2).
    pub fn indent(mut self, spaces: usize) -> Self {
        self.indent = spaces;
        self
    }
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for List {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pad = " ".repeat(self.indent);
        let num_width = if self.numbered {
            self.items.len().to_string().len()
        } else {
            0
        };

        for (i, item) in self.items.iter().enumerate() {
            let bullet = if self.numbered {
                format!("{}.", pad_num(i + 1, num_width))
            } else {
                format!("{}", theme::brand("\u{2022}")) // •
            };
            writeln!(f, "{}{} {}", pad, bullet, item)?;
        }
        Ok(())
    }
}

impl Renderable for List {
    fn line_count(&self) -> usize {
        self.items.len()
    }
}

/// Right-align a number to a given width.
fn pad_num(n: usize, width: usize) -> String {
    format!("{:>width$}", n, width = width)
}
