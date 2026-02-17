// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Block-level components for structured console output.
//!
//! Every component implements [`Renderable`] which provides:
//! - `line_count()` — how many terminal lines the component occupies
//! - `print()` — render to stdout, return a [`LiveHandle`] for clearing/refreshing
//! - `render()` — render to `String` (same as `Display`)
//!
//! # Live components
//!
//! [`LiveTable`] and [`LiveKeyValue`] support auto-refresh on mutation —
//! they clear and re-render when data changes, enabling live terminal dashboards.

mod card;
mod columns;
mod key_value;
mod list;
mod section;
mod table;

pub use card::Card;
pub use columns::Columns;
pub use key_value::{KeyValue, LiveKeyValue};
pub use list::List;
pub use section::Section;
pub use table::{LiveTable, Table};

use console::Term;
use std::fmt;
use std::io::Write;

/// Trait for all block components that can be rendered to terminal.
pub trait Renderable: fmt::Display {
    /// How many terminal lines this component occupies when rendered.
    fn line_count(&self) -> usize;

    /// Render to string (same as Display).
    fn render(&self) -> String {
        self.to_string()
    }

    /// Print to stdout and return a handle for clearing/refreshing.
    fn print(&self) -> LiveHandle {
        let rendered = self.render();
        let lines = self.line_count();
        let term = Term::stdout();
        // Prepend foreground SGR to each line so unstyled text uses the theme color
        let fg = crate::theme::foreground_sgr();
        let with_fg: String = rendered
            .lines()
            .map(|l| format!("{}{}", fg, l))
            .collect::<Vec<_>>()
            .join("\n");
        let _ = write!(&term, "{}", with_fg);
        let _ = term.flush();
        LiveHandle { lines, term }
    }
}

/// Handle returned by [`Renderable::print`] for clearing or refreshing output.
pub struct LiveHandle {
    lines: usize,
    term: Term,
}

impl LiveHandle {
    /// Create a new handle tracking a specific line count.
    pub fn new(lines: usize) -> Self {
        Self {
            lines,
            term: Term::stdout(),
        }
    }

    /// How many lines this handle tracks.
    pub fn lines(&self) -> usize {
        self.lines
    }

    /// Clear this component from the terminal.
    pub fn clear(&self) {
        if self.lines > 0 {
            let _ = self.term.clear_last_lines(self.lines);
        }
    }

    /// Clear the current output and re-render a new component in its place.
    pub fn refresh(self, new: &dyn Renderable) -> LiveHandle {
        self.clear();
        new.print()
    }
}

/// Measure the visible width of a string, ignoring ANSI escape codes.
pub(crate) fn visible_width(s: &str) -> usize {
    console::measure_text_width(s)
}

/// Pad a string to a target visible width (accounting for ANSI codes).
pub(crate) fn pad_visible(s: &str, target_width: usize) -> String {
    let current = visible_width(s);
    if current >= target_width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(target_width - current))
    }
}
