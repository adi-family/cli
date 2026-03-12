//! Terminal grid and cell types

use crate::color::Color;

/// Cell attributes (colors, bold, italic, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellAttributes {
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub inverse: bool,
    pub hidden: bool,
    pub dim: bool,
}

impl CellAttributes {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// A single cell in the terminal grid
#[derive(Debug, Clone, Copy, Default)]
pub struct Cell {
    pub c: char,
    pub attr: CellAttributes,
}

impl Cell {
    /// Create a new cell with a character and default attributes
    pub fn new(c: char) -> Self {
        Self {
            c,
            attr: CellAttributes::default(),
        }
    }

    /// Create a cell with character and attributes
    pub fn with_attr(c: char, attr: CellAttributes) -> Self {
        Self { c, attr }
    }
}

/// Terminal grid storing cells with scrollback
pub struct Grid {
    cells: Vec<Cell>,
    cols: usize,
    rows: usize,
    scrollback: Vec<Vec<Cell>>,
    scrollback_limit: usize,
    /// Current scroll offset (0 = at bottom, >0 = scrolled up into history)
    scroll_offset: usize,
}

impl Grid {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cells: vec![Cell::default(); cols * rows],
            cols,
            rows,
            scrollback: Vec::new(),
            scrollback_limit: 10000,
            scroll_offset: 0,
        }
    }

    /// Create with custom scrollback limit
    pub fn with_scrollback(cols: usize, rows: usize, scrollback_limit: usize) -> Self {
        Self {
            cells: vec![Cell::default(); cols * rows],
            cols,
            rows,
            scrollback: Vec::new(),
            scrollback_limit,
            scroll_offset: 0,
        }
    }

    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        let mut new_cells = vec![Cell::default(); new_cols * new_rows];

        for y in 0..new_rows.min(self.rows) {
            for x in 0..new_cols.min(self.cols) {
                new_cells[y * new_cols + x] = self.cells[y * self.cols + x];
            }
        }

        self.cells = new_cells;
        self.cols = new_cols;
        self.rows = new_rows;
    }

    pub fn get(&self, x: usize, y: usize) -> &Cell {
        if x < self.cols && y < self.rows {
            &self.cells[y * self.cols + x]
        } else {
            static DEFAULT: Cell = Cell {
                c: ' ',
                attr: CellAttributes {
                    fg: Color::Default,
                    bg: Color::Default,
                    bold: false,
                    italic: false,
                    underline: false,
                    strikethrough: false,
                    inverse: false,
                    hidden: false,
                    dim: false,
                },
            };
            &DEFAULT
        }
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> {
        if x < self.cols && y < self.rows {
            Some(&mut self.cells[y * self.cols + x])
        } else {
            None
        }
    }

    pub fn set(&mut self, x: usize, y: usize, c: char, attr: CellAttributes) {
        if x < self.cols && y < self.rows {
            self.cells[y * self.cols + x] = Cell { c, attr };
        }
    }

    pub fn clear_cell(&mut self, x: usize, y: usize) {
        if x < self.cols && y < self.rows {
            self.cells[y * self.cols + x] = Cell::default();
        }
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::default();
        }
    }

    pub fn scroll_up(&mut self, top: usize, bottom: usize, count: usize) {
        if top >= bottom || bottom >= self.rows {
            return;
        }

        // Save scrolled-out lines to scrollback if scrolling from top
        if top == 0 {
            for i in 0..count.min(bottom + 1) {
                let line: Vec<Cell> = (0..self.cols)
                    .map(|x| self.cells[i * self.cols + x])
                    .collect();
                self.scrollback.push(line);
            }
            while self.scrollback.len() > self.scrollback_limit {
                self.scrollback.remove(0);
            }
        }

        // Scroll up
        for y in top..=bottom.saturating_sub(count) {
            for x in 0..self.cols {
                if y + count <= bottom {
                    self.cells[y * self.cols + x] = self.cells[(y + count) * self.cols + x];
                }
            }
        }

        // Clear bottom lines
        for y in (bottom + 1).saturating_sub(count)..=bottom {
            for x in 0..self.cols {
                self.cells[y * self.cols + x] = Cell::default();
            }
        }
    }

    pub fn scroll_down(&mut self, top: usize, bottom: usize, count: usize) {
        if top >= bottom || bottom >= self.rows {
            return;
        }

        // Scroll down
        for y in (top + count..=bottom).rev() {
            for x in 0..self.cols {
                self.cells[y * self.cols + x] = self.cells[(y - count) * self.cols + x];
            }
        }

        // Clear top lines
        for y in top..top + count.min(bottom - top + 1) {
            for x in 0..self.cols {
                self.cells[y * self.cols + x] = Cell::default();
            }
        }
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    pub fn get_scrollback_line(&self, index: usize) -> Option<&Vec<Cell>> {
        self.scrollback.get(index)
    }

    /// Get current scroll offset (0 = at bottom)
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Scroll up by delta lines (view goes back in history)
    pub fn scroll_view_up(&mut self, delta: usize) {
        let max_offset = self.scrollback.len();
        self.scroll_offset = (self.scroll_offset + delta).min(max_offset);
    }

    /// Scroll down by delta lines (view goes forward toward present)
    pub fn scroll_view_down(&mut self, delta: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(delta);
    }

    /// Reset scroll to bottom (current content)
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Check if currently scrolled (not at bottom)
    pub fn is_scrolled(&self) -> bool {
        self.scroll_offset > 0
    }

    /// Get cell accounting for scroll offset
    pub fn get_with_scroll(&self, x: usize, y: usize) -> &Cell {
        if self.scroll_offset == 0 {
            return self.get(x, y);
        }

        let scrollback_len = self.scrollback.len();
        let visible_scrollback_start = scrollback_len.saturating_sub(self.scroll_offset);
        let history_line = visible_scrollback_start + y;

        if history_line < scrollback_len {
            let line = &self.scrollback[history_line];
            if x < line.len() {
                &line[x]
            } else {
                static DEFAULT: Cell = Cell {
                    c: ' ',
                    attr: CellAttributes {
                        fg: Color::Default,
                        bg: Color::Default,
                        bold: false,
                        italic: false,
                        underline: false,
                        strikethrough: false,
                        inverse: false,
                        hidden: false,
                        dim: false,
                    },
                };
                &DEFAULT
            }
        } else {
            let grid_line = history_line - scrollback_len;
            self.get(x, grid_line)
        }
    }

    /// Get a line as a string (trimming trailing spaces)
    pub fn line_to_string(&self, y: usize) -> String {
        if y >= self.rows {
            return String::new();
        }
        let mut s: String = (0..self.cols).map(|x| self.get(x, y).c).collect();
        // Trim trailing spaces
        while s.ends_with(' ') {
            s.pop();
        }
        s
    }
}
