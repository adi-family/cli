//! Terminal state machine

use crate::grid::{CellAttributes, Grid};
use parking_lot::Mutex;
use std::sync::Arc;

/// Selection state for text selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// Starting point (where mouse down occurred) (x, y)
    pub start: (usize, usize),
    /// Current end point (where mouse is/was) (x, y)
    pub end: (usize, usize),
}

impl Selection {
    /// Get normalized selection (start before end)
    pub fn normalized(&self) -> ((usize, usize), (usize, usize)) {
        let (start_y, start_x) = (self.start.1, self.start.0);
        let (end_y, end_x) = (self.end.1, self.end.0);

        if start_y < end_y || (start_y == end_y && start_x <= end_x) {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    /// Check if a cell is within the selection
    pub fn contains(&self, x: usize, y: usize) -> bool {
        let ((start_x, start_y), (end_x, end_y)) = self.normalized();

        if y < start_y || y > end_y {
            return false;
        }

        if y == start_y && y == end_y {
            x >= start_x && x <= end_x
        } else if y == start_y {
            x >= start_x
        } else if y == end_y {
            x <= end_x
        } else {
            true
        }
    }
}

/// Terminal state containing the grid and cursor position
pub struct Terminal {
    pub grid: Grid,
    pub cols: usize,
    pub rows: usize,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub cursor_visible: bool,
    pub saved_cursor: Option<(usize, usize)>,
    pub current_attr: CellAttributes,
    pub scroll_top: usize,
    pub scroll_bottom: usize,
    pub mode_origin: bool,
    pub mode_autowrap: bool,
    pub title: String,
    dirty: bool,
    /// Visual bell: Some(start_time) when bell is active
    pub bell_active: Option<std::time::Instant>,
    /// Current text selection
    pub selection: Option<Selection>,
}

impl Terminal {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            grid: Grid::new(cols, rows),
            cols,
            rows,
            cursor_x: 0,
            cursor_y: 0,
            cursor_visible: true,
            saved_cursor: None,
            current_attr: CellAttributes::default(),
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            mode_origin: false,
            mode_autowrap: true,
            title: String::from("Terminal"),
            dirty: true,
            bell_active: None,
            selection: None,
        }
    }

    /// Start a new selection at the given cell coordinates
    pub fn start_selection(&mut self, x: usize, y: usize) {
        self.selection = Some(Selection {
            start: (x, y),
            end: (x, y),
        });
        self.dirty = true;
    }

    /// Update the selection end point
    pub fn update_selection(&mut self, x: usize, y: usize) {
        if let Some(ref mut sel) = self.selection {
            sel.end = (x, y);
            self.dirty = true;
        }
    }

    /// Clear the current selection
    pub fn clear_selection(&mut self) {
        if self.selection.is_some() {
            self.selection = None;
            self.dirty = true;
        }
    }

    /// Get the selected text as a string
    pub fn get_selected_text(&self) -> Option<String> {
        let sel = self.selection?;
        let ((start_x, start_y), (end_x, end_y)) = sel.normalized();

        let mut result = String::new();

        for y in start_y..=end_y {
            let line_start = if y == start_y { start_x } else { 0 };
            let line_end = if y == end_y { end_x } else { self.cols - 1 };

            for x in line_start..=line_end {
                let cell = self.grid.get_with_scroll(x, y);
                if cell.c != '\0' && cell.c != ' ' {
                    result.push(cell.c);
                } else if cell.c == ' ' {
                    result.push(' ');
                }
            }

            if y != end_y {
                while result.ends_with(' ') {
                    result.pop();
                }
                result.push('\n');
            }
        }

        while result.ends_with(' ') {
            result.pop();
        }

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Trigger visual bell
    pub fn ring_bell(&mut self) {
        self.bell_active = Some(std::time::Instant::now());
        self.dirty = true;
    }

    /// Check if bell is still visible (200ms duration)
    pub fn is_bell_visible(&self) -> bool {
        if let Some(start) = self.bell_active {
            start.elapsed().as_millis() < 200
        } else {
            false
        }
    }

    /// Clear expired bell state
    pub fn update_bell(&mut self) {
        if let Some(start) = self.bell_active {
            if start.elapsed().as_millis() >= 200 {
                self.bell_active = None;
            }
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.grid.resize(cols, rows);
        self.cols = cols;
        self.rows = rows;
        self.scroll_bottom = rows.saturating_sub(1);
        self.cursor_x = self.cursor_x.min(cols.saturating_sub(1));
        self.cursor_y = self.cursor_y.min(rows.saturating_sub(1));
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn put_char(&mut self, c: char) {
        if self.cursor_x >= self.cols {
            if self.mode_autowrap {
                self.cursor_x = 0;
                self.linefeed();
            } else {
                self.cursor_x = self.cols.saturating_sub(1);
            }
        }

        self.grid
            .set(self.cursor_x, self.cursor_y, c, self.current_attr);
        self.cursor_x += 1;
        self.dirty = true;
    }

    pub fn carriage_return(&mut self) {
        self.cursor_x = 0;
    }

    pub fn linefeed(&mut self) {
        if self.cursor_y >= self.scroll_bottom {
            self.scroll_up(1);
        } else {
            self.cursor_y += 1;
        }
        self.dirty = true;
    }

    pub fn backspace(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        }
    }

    pub fn tab(&mut self) {
        let next_tab = ((self.cursor_x / 8) + 1) * 8;
        self.cursor_x = next_tab.min(self.cols.saturating_sub(1));
    }

    pub fn scroll_up(&mut self, count: usize) {
        self.grid
            .scroll_up(self.scroll_top, self.scroll_bottom, count);
        self.dirty = true;
    }

    pub fn scroll_down(&mut self, count: usize) {
        self.grid
            .scroll_down(self.scroll_top, self.scroll_bottom, count);
        self.dirty = true;
    }

    pub fn move_cursor_to(&mut self, x: usize, y: usize) {
        self.cursor_x = x.min(self.cols.saturating_sub(1));
        self.cursor_y = y.min(self.rows.saturating_sub(1));
    }

    pub fn move_cursor_up(&mut self, count: usize) {
        self.cursor_y = self.cursor_y.saturating_sub(count);
    }

    pub fn move_cursor_down(&mut self, count: usize) {
        self.cursor_y = (self.cursor_y + count).min(self.rows.saturating_sub(1));
    }

    pub fn move_cursor_forward(&mut self, count: usize) {
        self.cursor_x = (self.cursor_x + count).min(self.cols.saturating_sub(1));
    }

    pub fn move_cursor_backward(&mut self, count: usize) {
        self.cursor_x = self.cursor_x.saturating_sub(count);
    }

    pub fn erase_in_display(&mut self, mode: u16) {
        match mode {
            0 => {
                for x in self.cursor_x..self.cols {
                    self.grid.clear_cell(x, self.cursor_y);
                }
                for y in (self.cursor_y + 1)..self.rows {
                    for x in 0..self.cols {
                        self.grid.clear_cell(x, y);
                    }
                }
            }
            1 => {
                for y in 0..self.cursor_y {
                    for x in 0..self.cols {
                        self.grid.clear_cell(x, y);
                    }
                }
                for x in 0..=self.cursor_x {
                    self.grid.clear_cell(x, self.cursor_y);
                }
            }
            2 | 3 => {
                self.grid.clear();
            }
            _ => {}
        }
        self.dirty = true;
    }

    pub fn erase_in_line(&mut self, mode: u16) {
        match mode {
            0 => {
                for x in self.cursor_x..self.cols {
                    self.grid.clear_cell(x, self.cursor_y);
                }
            }
            1 => {
                for x in 0..=self.cursor_x.min(self.cols.saturating_sub(1)) {
                    self.grid.clear_cell(x, self.cursor_y);
                }
            }
            2 => {
                for x in 0..self.cols {
                    self.grid.clear_cell(x, self.cursor_y);
                }
            }
            _ => {}
        }
        self.dirty = true;
    }

    pub fn insert_lines(&mut self, count: usize) {
        if self.cursor_y >= self.scroll_top && self.cursor_y <= self.scroll_bottom {
            self.grid
                .scroll_down(self.cursor_y, self.scroll_bottom, count);
        }
        self.dirty = true;
    }

    pub fn delete_lines(&mut self, count: usize) {
        if self.cursor_y >= self.scroll_top && self.cursor_y <= self.scroll_bottom {
            self.grid
                .scroll_up(self.cursor_y, self.scroll_bottom, count);
        }
        self.dirty = true;
    }

    pub fn delete_chars(&mut self, count: usize) {
        let count = count.min(self.cols.saturating_sub(self.cursor_x));
        for x in self.cursor_x..(self.cols.saturating_sub(count)) {
            let cell = *self.grid.get(x + count, self.cursor_y);
            self.grid.set(x, self.cursor_y, cell.c, cell.attr);
        }
        for x in (self.cols.saturating_sub(count))..self.cols {
            self.grid.clear_cell(x, self.cursor_y);
        }
        self.dirty = true;
    }

    pub fn insert_blank(&mut self, count: usize) {
        let count = count.min(self.cols.saturating_sub(self.cursor_x));
        for x in (self.cursor_x..self.cols.saturating_sub(count)).rev() {
            let cell = *self.grid.get(x, self.cursor_y);
            self.grid.set(x + count, self.cursor_y, cell.c, cell.attr);
        }
        for x in self.cursor_x..(self.cursor_x + count).min(self.cols) {
            self.grid.clear_cell(x, self.cursor_y);
        }
        self.dirty = true;
    }

    pub fn save_cursor(&mut self) {
        self.saved_cursor = Some((self.cursor_x, self.cursor_y));
    }

    pub fn restore_cursor(&mut self) {
        if let Some((x, y)) = self.saved_cursor {
            self.cursor_x = x;
            self.cursor_y = y;
        }
    }

    pub fn set_scrolling_region(&mut self, top: usize, bottom: usize) {
        let top = top.saturating_sub(1);
        let bottom = (bottom.saturating_sub(1)).min(self.rows.saturating_sub(1));
        if top < bottom {
            self.scroll_top = top;
            self.scroll_bottom = bottom;
        }
    }

    pub fn reset(&mut self) {
        self.grid.clear();
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.cursor_visible = true;
        self.current_attr = CellAttributes::default();
        self.scroll_top = 0;
        self.scroll_bottom = self.rows.saturating_sub(1);
        self.mode_origin = false;
        self.mode_autowrap = true;
        self.bell_active = None;
        self.selection = None;
        self.dirty = true;
    }
}

pub type SharedTerminal = Arc<Mutex<Terminal>>;

pub fn create_shared_terminal(cols: usize, rows: usize) -> SharedTerminal {
    Arc::new(Mutex::new(Terminal::new(cols, rows)))
}
