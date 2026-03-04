// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Card (bordered panel) component.
//!
//! ```text
//! ╭─ Daemon Status ──────────────╮
//! │  PID:       1234              │
//! │  Version:   0.5.0             │
//! │  Uptime:    3600s             │
//! ╰──────────────────────────────╯
//! ```

use super::{visible_width, Renderable};
use crate::theme;
use std::fmt;

/// Bordered card with optional title.
pub struct Card {
    title: Option<String>,
    lines: Vec<String>,
    width: Option<usize>,
    padding: usize,
}

impl Card {
    /// Create a new empty card.
    pub fn new() -> Self {
        Self {
            title: None,
            lines: Vec::new(),
            width: None,
            padding: 1,
        }
    }

    /// Set the card title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add a line of content.
    pub fn line(mut self, text: impl Into<String>) -> Self {
        self.lines.push(text.into());
        self
    }

    /// Add multiple lines of content.
    pub fn lines<I, S>(mut self, lines: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.lines.extend(lines.into_iter().map(Into::into));
        self
    }

    /// Set a fixed width. If not set, auto-sizes to content.
    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Set internal padding (spaces on each side, default: 1).
    pub fn padding(mut self, padding: usize) -> Self {
        self.padding = padding;
        self
    }

    /// Calculate the inner width (content area).
    fn inner_width(&self) -> usize {
        let content_max = self
            .lines
            .iter()
            .map(|l| visible_width(l))
            .max()
            .unwrap_or(0);

        let title_width = self
            .title
            .as_ref()
            .map(|t| visible_width(t) + 4) // "─ Title ─" adds 4 chars
            .unwrap_or(0);

        let auto_width = content_max.max(title_width) + self.padding * 2;

        self.width.unwrap_or(auto_width)
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = self.inner_width();
        let h = "\u{2500}"; // ─

        // Top border — style border chars individually to not override title styling
        if let Some(ref title) = self.title {
            let title_str = format!(" {} ", theme::brand_bold(title));
            let title_visible = visible_width(&title_str);
            let remaining = inner.saturating_sub(1 + title_visible);
            writeln!(
                f,
                "{}{}{}{}{}",
                theme::muted("\u{256D}"),
                theme::muted(h),
                title_str,
                theme::muted(h.repeat(remaining)),
                theme::muted("\u{256E}")
            )?;
        } else {
            writeln!(
                f,
                "{}{}{}",
                theme::muted("\u{256D}"),
                theme::muted(h.repeat(inner)),
                theme::muted("\u{256E}")
            )?;
        }

        // Content lines
        let pad = " ".repeat(self.padding);
        for line in &self.lines {
            let line_visible = visible_width(line);
            let right_pad = inner.saturating_sub(self.padding * 2 + line_visible);
            writeln!(
                f,
                "{}{}{}{} {}",
                theme::muted("\u{2502}"),
                pad,
                line,
                " ".repeat(right_pad),
                theme::muted("\u{2502}")
            )?;
        }

        // Bottom border
        writeln!(
            f,
            "{}{}{}",
            theme::muted("\u{2570}"),
            theme::muted(h.repeat(inner)),
            theme::muted("\u{256F}")
        )?;

        Ok(())
    }
}

impl Renderable for Card {
    fn line_count(&self) -> usize {
        self.lines.len() + 2 // top + bottom borders
    }
}
