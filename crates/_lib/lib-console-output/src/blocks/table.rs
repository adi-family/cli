// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Bordered table and live table components.
//!
//! ```text
//! ╭──────────┬──────────┬──────╮
//! │ Service  │ Status   │ Port │
//! ├──────────┼──────────┼──────┤
//! │ web      │ running  │ 8080 │
//! │ api      │ stopped  │ 3000 │
//! ╰──────────┴──────────┴──────╯
//! ```

use super::{pad_visible, visible_width, LiveHandle, Renderable};
use crate::theme;
use console::Term;
use std::fmt;
use std::io::Write;

/// Bordered table with optional header.
pub struct Table {
    header: Option<Vec<String>>,
    rows: Vec<Vec<String>>,
    padding: usize,
}

impl Table {
    /// Create a new empty table.
    pub fn new() -> Self {
        Self {
            header: None,
            rows: Vec::new(),
            padding: 1,
        }
    }

    /// Set the header row.
    pub fn header<I, S>(mut self, cols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.header = Some(cols.into_iter().map(Into::into).collect());
        self
    }

    /// Add a data row.
    pub fn row<I, S>(mut self, cols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.rows.push(cols.into_iter().map(Into::into).collect());
        self
    }

    /// Add multiple rows at once.
    pub fn rows<R, I, S>(mut self, rows: R) -> Self
    where
        R: IntoIterator<Item = I>,
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for row in rows {
            self.rows.push(row.into_iter().map(Into::into).collect());
        }
        self
    }

    /// Set cell padding (spaces on each side, default: 1).
    pub fn padding(mut self, padding: usize) -> Self {
        self.padding = padding;
        self
    }

    /// Calculate the number of columns.
    fn col_count(&self) -> usize {
        let header_count = self.header.as_ref().map_or(0, |h| h.len());
        let row_max = self.rows.iter().map(|r| r.len()).max().unwrap_or(0);
        header_count.max(row_max)
    }

    /// Calculate column widths (content only, without padding).
    fn col_widths(&self) -> Vec<usize> {
        let cols = self.col_count();
        let mut widths = vec![0usize; cols];

        if let Some(ref header) = self.header {
            for (i, cell) in header.iter().enumerate() {
                widths[i] = widths[i].max(visible_width(cell));
            }
        }

        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < cols {
                    widths[i] = widths[i].max(visible_width(cell));
                }
            }
        }

        widths
    }

    /// Format a horizontal border line.
    fn border_line(&self, widths: &[usize], left: &str, mid: &str, right: &str) -> String {
        let h = "\u{2500}"; // ─
        let segments: Vec<String> = widths
            .iter()
            .map(|&w| h.repeat(w + self.padding * 2))
            .collect();
        theme::muted(format!("{}{}{}", left, segments.join(mid), right)).to_string()
    }

    /// Format a data row.
    fn format_data_row(&self, cells: &[String], widths: &[usize]) -> String {
        let v = theme::muted("\u{2502}").to_string(); // │
        let pad = " ".repeat(self.padding);

        let formatted: Vec<String> = widths
            .iter()
            .enumerate()
            .map(|(i, &w)| {
                let cell = cells.get(i).map(|s| s.as_str()).unwrap_or("");
                format!("{}{}{}", pad, pad_visible(cell, w), pad)
            })
            .collect();

        format!("{}{}{}", v, formatted.join(&v), v)
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let widths = self.col_widths();

        // Top border: ╭──┬──╮
        writeln!(
            f,
            "{}",
            self.border_line(&widths, "\u{256D}", "\u{252C}", "\u{256E}")
        )?;

        // Header row
        if let Some(ref header) = self.header {
            let styled: Vec<String> = header
                .iter()
                .map(|cell| theme::bold(cell).to_string())
                .collect();
            writeln!(f, "{}", self.format_data_row(&styled, &widths))?;

            // Header separator: ├──┼──┤
            writeln!(
                f,
                "{}",
                self.border_line(&widths, "\u{251C}", "\u{253C}", "\u{2524}")
            )?;
        }

        // Data rows
        for row in &self.rows {
            writeln!(f, "{}", self.format_data_row(row, &widths))?;
        }

        // Bottom border: ╰──┴──╯
        writeln!(
            f,
            "{}",
            self.border_line(&widths, "\u{2570}", "\u{2534}", "\u{256F}")
        )?;

        Ok(())
    }
}

impl Renderable for Table {
    fn line_count(&self) -> usize {
        let header_lines = if self.header.is_some() { 2 } else { 0 }; // header row + separator
        2 + header_lines + self.rows.len() // top + bottom borders
    }
}

/// Live table that auto-refreshes when rows are added or updated.
pub struct LiveTable {
    header: Option<Vec<String>>,
    rows: Vec<Vec<String>>,
    padding: usize,
    rendered_lines: usize,
    term: Term,
}

impl LiveTable {
    /// Create a new live table.
    pub fn new() -> Self {
        Self {
            header: None,
            rows: Vec::new(),
            padding: 1,
            rendered_lines: 0,
            term: Term::stdout(),
        }
    }

    /// Set the header row.
    pub fn header<I, S>(mut self, cols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.header = Some(cols.into_iter().map(Into::into).collect());
        self
    }

    /// Set cell padding (default: 1).
    pub fn padding(mut self, padding: usize) -> Self {
        self.padding = padding;
        self
    }

    /// Add a new row and re-render.
    pub fn push_row<I, S>(&mut self, cols: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.rows.push(cols.into_iter().map(Into::into).collect());
        self.rerender();
    }

    /// Update an existing row by index and re-render.
    /// If index is out of bounds, does nothing.
    pub fn set_row<I, S>(&mut self, index: usize, cols: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        if index < self.rows.len() {
            self.rows[index] = cols.into_iter().map(Into::into).collect();
            self.rerender();
        }
    }

    /// Remove a row by index and re-render.
    pub fn remove_row(&mut self, index: usize) {
        if index < self.rows.len() {
            self.rows.remove(index);
            self.rerender();
        }
    }

    /// Get the current number of rows.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Finalize the table — output stays, no more clearing.
    pub fn done(self) -> LiveHandle {
        LiveHandle::new(self.rendered_lines)
    }

    /// Build a static Table from current state.
    fn to_static(&self) -> Table {
        let mut table = Table::new().padding(self.padding);
        if let Some(ref h) = self.header {
            table.header = Some(h.clone());
        }
        table.rows = self.rows.clone();
        table
    }

    /// Clear and re-render.
    fn rerender(&mut self) {
        if self.rendered_lines > 0 {
            let _ = self.term.clear_last_lines(self.rendered_lines);
        }

        let static_table = self.to_static();
        let rendered = static_table.render();
        self.rendered_lines = static_table.line_count();

        let _ = write!(&self.term, "{}", rendered);
        let _ = self.term.flush();
    }
}

impl Default for LiveTable {
    fn default() -> Self {
        Self::new()
    }
}
