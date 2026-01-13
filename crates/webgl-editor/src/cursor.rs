//! Cursor and Selection Management
//!
//! Handles cursor positioning, movement, and text selection.

/// Represents a text selection range
#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Selection {
    /// Create a normalized selection (start before end)
    pub fn normalized(&self) -> Selection {
        if self.start_line > self.end_line
            || (self.start_line == self.end_line && self.start_col > self.end_col)
        {
            Selection {
                start_line: self.end_line,
                start_col: self.end_col,
                end_line: self.start_line,
                end_col: self.start_col,
            }
        } else {
            *self
        }
    }

    /// Check if selection is empty (cursor only)
    pub fn is_empty(&self) -> bool {
        self.start_line == self.end_line && self.start_col == self.end_col
    }
}

/// Cursor state including position and optional selection
pub struct Cursor {
    line: usize,
    col: usize,
    /// Selection anchor (where selection started)
    anchor_line: Option<usize>,
    anchor_col: Option<usize>,
    /// Preferred column for vertical movement
    preferred_col: Option<usize>,
}

impl Cursor {
    pub fn new() -> Self {
        Cursor {
            line: 0,
            col: 0,
            anchor_line: None,
            anchor_col: None,
            preferred_col: None,
        }
    }

    /// Get current line (0-indexed)
    pub fn line(&self) -> usize {
        self.line
    }

    /// Get current column (0-indexed)
    pub fn col(&self) -> usize {
        self.col
    }

    /// Move cursor to specific position, clearing selection
    pub fn move_to(&mut self, line: usize, col: usize) {
        self.line = line;
        self.col = col;
        self.anchor_line = None;
        self.anchor_col = None;
        self.preferred_col = None;
    }

    /// Move cursor left
    pub fn move_left(&mut self, line_len: impl Fn(usize) -> usize) {
        if self.col > 0 {
            self.col -= 1;
        } else if self.line > 0 {
            self.line -= 1;
            self.col = line_len(self.line);
        }
        self.clear_selection();
        self.preferred_col = None;
    }

    /// Move cursor right
    pub fn move_right(&mut self, line_len: impl Fn(usize) -> usize, line_count: usize) {
        let current_len = line_len(self.line);
        if self.col < current_len {
            self.col += 1;
        } else if self.line < line_count.saturating_sub(1) {
            self.line += 1;
            self.col = 0;
        }
        self.clear_selection();
        self.preferred_col = None;
    }

    /// Move cursor up
    pub fn move_up(&mut self, line_len: impl Fn(usize) -> usize) {
        if self.line > 0 {
            let preferred = self.preferred_col.unwrap_or(self.col);
            self.line -= 1;
            self.col = preferred.min(line_len(self.line));
            self.preferred_col = Some(preferred);
        }
        self.clear_selection();
    }

    /// Move cursor down
    pub fn move_down(&mut self, line_len: impl Fn(usize) -> usize, line_count: usize) {
        if self.line < line_count.saturating_sub(1) {
            let preferred = self.preferred_col.unwrap_or(self.col);
            self.line += 1;
            self.col = preferred.min(line_len(self.line));
            self.preferred_col = Some(preferred);
        }
        self.clear_selection();
    }

    /// Move to start of line
    pub fn move_to_line_start(&mut self) {
        self.col = 0;
        self.clear_selection();
        self.preferred_col = None;
    }

    /// Move to end of line
    pub fn move_to_line_end(&mut self, line_len: usize) {
        self.col = line_len;
        self.clear_selection();
        self.preferred_col = None;
    }

    /// Move to start of document
    pub fn move_to_start(&mut self) {
        self.line = 0;
        self.col = 0;
        self.clear_selection();
        self.preferred_col = None;
    }

    /// Move to end of document
    pub fn move_to_end(&mut self, line_count: usize, last_line_len: usize) {
        self.line = line_count.saturating_sub(1);
        self.col = last_line_len;
        self.clear_selection();
        self.preferred_col = None;
    }

    /// Start selection from current position
    pub fn start_selection(&mut self) {
        self.anchor_line = Some(self.line);
        self.anchor_col = Some(self.col);
    }

    /// Extend selection to position
    pub fn select_to(&mut self, line: usize, col: usize) {
        if self.anchor_line.is_none() {
            self.anchor_line = Some(self.line);
            self.anchor_col = Some(self.col);
        }
        self.line = line;
        self.col = col;
    }

    /// Select left
    pub fn select_left(&mut self, line_len: impl Fn(usize) -> usize) {
        if self.anchor_line.is_none() {
            self.start_selection();
        }
        if self.col > 0 {
            self.col -= 1;
        } else if self.line > 0 {
            self.line -= 1;
            self.col = line_len(self.line);
        }
        self.preferred_col = None;
    }

    /// Select right
    pub fn select_right(&mut self, line_len: impl Fn(usize) -> usize, line_count: usize) {
        if self.anchor_line.is_none() {
            self.start_selection();
        }
        let current_len = line_len(self.line);
        if self.col < current_len {
            self.col += 1;
        } else if self.line < line_count.saturating_sub(1) {
            self.line += 1;
            self.col = 0;
        }
        self.preferred_col = None;
    }

    /// Select up
    pub fn select_up(&mut self, line_len: impl Fn(usize) -> usize) {
        if self.anchor_line.is_none() {
            self.start_selection();
        }
        if self.line > 0 {
            let preferred = self.preferred_col.unwrap_or(self.col);
            self.line -= 1;
            self.col = preferred.min(line_len(self.line));
            self.preferred_col = Some(preferred);
        }
    }

    /// Select down
    pub fn select_down(&mut self, line_len: impl Fn(usize) -> usize, line_count: usize) {
        if self.anchor_line.is_none() {
            self.start_selection();
        }
        if self.line < line_count.saturating_sub(1) {
            let preferred = self.preferred_col.unwrap_or(self.col);
            self.line += 1;
            self.col = preferred.min(line_len(self.line));
            self.preferred_col = Some(preferred);
        }
    }

    /// Select all
    pub fn select_all(&mut self, line_count: usize, last_line_len: usize) {
        self.anchor_line = Some(0);
        self.anchor_col = Some(0);
        self.line = line_count.saturating_sub(1);
        self.col = last_line_len;
    }

    /// Get current selection if any
    pub fn selection(&self) -> Option<Selection> {
        if let (Some(anchor_line), Some(anchor_col)) = (self.anchor_line, self.anchor_col) {
            let sel = Selection {
                start_line: anchor_line,
                start_col: anchor_col,
                end_line: self.line,
                end_col: self.col,
            };
            if !sel.is_empty() {
                return Some(sel.normalized());
            }
        }
        None
    }

    /// Check if there is an active selection
    pub fn has_selection(&self) -> bool {
        self.selection().is_some()
    }

    /// Clear current selection
    pub fn clear_selection(&mut self) {
        self.anchor_line = None;
        self.anchor_col = None;
    }

    /// Reset cursor to beginning
    pub fn reset(&mut self) {
        self.line = 0;
        self.col = 0;
        self.anchor_line = None;
        self.anchor_col = None;
        self.preferred_col = None;
    }

    /// Set position after an edit operation
    pub fn set_position(&mut self, line: usize, col: usize) {
        self.line = line;
        self.col = col;
        self.preferred_col = None;
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}
