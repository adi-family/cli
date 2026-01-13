//! Viewport Management
//!
//! Handles scrolling and determines which portion of the document is visible.

/// Manages the visible viewport of the editor
pub struct Viewport {
    /// First visible line (0-indexed)
    scroll_line: usize,
    /// First visible column (0-indexed, for horizontal scroll)
    scroll_col: usize,
    /// Number of lines visible in viewport
    visible_lines: usize,
    /// Number of columns visible in viewport
    visible_cols: usize,
    /// Scroll margin (lines to keep visible around cursor)
    scroll_margin: usize,
}

impl Viewport {
    pub fn new(visible_lines: usize, visible_cols: usize) -> Self {
        Viewport {
            scroll_line: 0,
            scroll_col: 0,
            visible_lines,
            visible_cols,
            scroll_margin: 3,
        }
    }

    /// Get the first visible line
    pub fn scroll_line(&self) -> usize {
        self.scroll_line
    }

    /// Get the first visible column
    pub fn scroll_col(&self) -> usize {
        self.scroll_col
    }

    /// Get number of visible lines
    pub fn visible_lines(&self) -> usize {
        self.visible_lines
    }

    /// Get number of visible columns
    pub fn visible_cols(&self) -> usize {
        self.visible_cols
    }

    /// Set number of visible lines (on resize)
    pub fn set_visible_lines(&mut self, lines: usize) {
        self.visible_lines = lines;
    }

    /// Set number of visible columns (on resize)
    pub fn set_visible_cols(&mut self, cols: usize) {
        self.visible_cols = cols;
    }

    /// Scroll to a specific position
    pub fn scroll_to(&mut self, line: usize, col: usize) {
        self.scroll_line = line;
        self.scroll_col = col;
    }

    /// Scroll by a delta amount
    pub fn scroll_by(&mut self, delta_lines: i32, delta_cols: i32, max_lines: usize) {
        // Vertical scroll
        if delta_lines < 0 {
            self.scroll_line = self.scroll_line.saturating_sub((-delta_lines) as usize);
        } else {
            self.scroll_line =
                (self.scroll_line + delta_lines as usize).min(max_lines.saturating_sub(1));
        }

        // Horizontal scroll
        if delta_cols < 0 {
            self.scroll_col = self.scroll_col.saturating_sub((-delta_cols) as usize);
        } else {
            self.scroll_col += delta_cols as usize;
        }
    }

    /// Ensure a position is visible (with margin)
    pub fn ensure_visible(&mut self, line: usize, col: usize, max_lines: usize) {
        // Vertical scrolling
        let margin = self.scroll_margin.min(self.visible_lines / 2);

        // Scroll up if cursor is above viewport
        if line < self.scroll_line + margin {
            self.scroll_line = line.saturating_sub(margin);
        }

        // Scroll down if cursor is below viewport
        let bottom_margin = self.visible_lines.saturating_sub(margin).saturating_sub(1);
        if line >= self.scroll_line + bottom_margin {
            self.scroll_line = (line.saturating_sub(bottom_margin))
                .min(max_lines.saturating_sub(self.visible_lines));
        }

        // Horizontal scrolling
        let h_margin = 5;

        // Scroll left if cursor is before viewport
        if col < self.scroll_col + h_margin {
            self.scroll_col = col.saturating_sub(h_margin);
        }

        // Scroll right if cursor is after viewport
        let right_margin = self.visible_cols.saturating_sub(h_margin);
        if col >= self.scroll_col + right_margin {
            self.scroll_col = col.saturating_sub(right_margin);
        }
    }

    /// Check if a line is visible
    pub fn is_line_visible(&self, line: usize) -> bool {
        line >= self.scroll_line && line < self.scroll_line + self.visible_lines
    }

    /// Check if a position is visible
    pub fn is_visible(&self, line: usize, col: usize) -> bool {
        self.is_line_visible(line)
            && col >= self.scroll_col
            && col < self.scroll_col + self.visible_cols
    }
}
