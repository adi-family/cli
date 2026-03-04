// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Section header component.
//!
//! ```text
//! ── Services ──────────────────────
//! ```

use super::{visible_width, Renderable};
use crate::theme;
use std::fmt;

/// Section header with a title and separator line.
pub struct Section {
    title: String,
    width: usize,
}

impl Section {
    /// Create a new section header.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 40,
        }
    }

    /// Set the total width of the separator line.
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }
}

impl fmt::Display for Section {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let title_str = format!(" {} ", theme::brand_bold(&self.title));
        let title_visible = visible_width(&title_str);
        let dash = "\u{2500}"; // ─

        let left = format!("{}{}", theme::muted(dash), theme::muted(dash));
        let remaining = self.width.saturating_sub(2 + title_visible);
        let right = theme::muted(dash.repeat(remaining));

        writeln!(f, "{}{}{}", left, title_str, right)
    }
}

impl Renderable for Section {
    fn line_count(&self) -> usize {
        1
    }
}
