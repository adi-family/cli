// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Borderless aligned columns component.
//!
//! ```text
//!   SERVICE   STATUS    PORT
//!   ────────  ────────  ──────
//!   web       running   8080
//!   api       stopped   3000
//! ```

use super::{pad_visible, visible_width, Renderable};
use crate::theme;
use std::fmt;

/// Borderless aligned columns.
pub struct Columns {
    header: Option<Vec<String>>,
    rows: Vec<Vec<String>>,
    gap: usize,
    indent: usize,
}

impl Columns {
    /// Create new empty columns.
    pub fn new() -> Self {
        Self {
            header: None,
            rows: Vec::new(),
            gap: 2,
            indent: 2,
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

    /// Set the gap between columns (default: 2).
    pub fn gap(mut self, gap: usize) -> Self {
        self.gap = gap;
        self
    }

    /// Set indentation (default: 2).
    pub fn indent(mut self, spaces: usize) -> Self {
        self.indent = spaces;
        self
    }

    /// Calculate the number of columns.
    fn col_count(&self) -> usize {
        let header_count = self.header.as_ref().map_or(0, |h| h.len());
        let row_max = self.rows.iter().map(|r| r.len()).max().unwrap_or(0);
        header_count.max(row_max)
    }

    /// Calculate column widths.
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

    /// Format a row with proper alignment.
    fn format_row(&self, cells: &[String], widths: &[usize]) -> String {
        let pad = " ".repeat(self.indent);
        let gap = " ".repeat(self.gap);

        let formatted: Vec<String> = widths
            .iter()
            .enumerate()
            .map(|(i, &w)| {
                let cell = cells.get(i).map(|s| s.as_str()).unwrap_or("");
                pad_visible(cell, w)
            })
            .collect();

        format!("{}{}", pad, formatted.join(&gap))
    }
}

impl Default for Columns {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Columns {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let widths = self.col_widths();
        let pad = " ".repeat(self.indent);
        let gap = " ".repeat(self.gap);

        // Header
        if let Some(ref header) = self.header {
            let header_styled: Vec<String> = header
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let w = widths.get(i).copied().unwrap_or(0);
                    pad_visible(&theme::bold(cell).to_string(), w)
                })
                .collect();
            writeln!(f, "{}{}", pad, header_styled.join(&gap))?;

            // Underline separator
            let separators: Vec<String> = widths
                .iter()
                .map(|&w| theme::muted("\u{2500}".repeat(w)).to_string())
                .collect();
            writeln!(f, "{}{}", pad, separators.join(&gap))?;
        }

        // Data rows
        for row in &self.rows {
            writeln!(f, "{}", self.format_row(row, &widths))?;
        }

        Ok(())
    }
}

impl Renderable for Columns {
    fn line_count(&self) -> usize {
        let header_lines = if self.header.is_some() { 2 } else { 0 };
        header_lines + self.rows.len()
    }
}
