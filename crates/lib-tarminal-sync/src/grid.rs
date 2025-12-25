//! Terminal grid synchronization
//!
//! Delta operations and snapshots for efficient terminal state sync.

use serde::{Deserialize, Serialize};

/// Terminal cell with character and styling
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    pub char: char,
    pub fg: TerminalColor,
    pub bg: TerminalColor,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            char: ' ',
            fg: TerminalColor::Default,
            bg: TerminalColor::Default,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            inverse: false,
            hidden: false,
            strikethrough: false,
        }
    }
}

/// Terminal color types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalColor {
    Default,
    Named { color: NamedColor },
    Indexed { index: u8 },
    Rgb { r: u8, g: u8, b: u8 },
}

/// Named ANSI colors (0-15)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamedColor {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
    BrightBlack = 8,
    BrightRed = 9,
    BrightGreen = 10,
    BrightYellow = 11,
    BrightBlue = 12,
    BrightMagenta = 13,
    BrightCyan = 14,
    BrightWhite = 15,
}

/// Delta operations for incremental grid sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridDelta {
    pub operations: Vec<GridOperation>,
    pub base_version: u64,
    pub new_version: u64,
}

/// Individual grid operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum GridOperation {
    SetCells {
        row: usize,
        start_col: usize,
        cells: Vec<Cell>,
    },
    ScrollUp {
        lines: usize,
    },
    ScrollDown {
        lines: usize,
    },
    ClearRegion {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    },
    Resize {
        cols: usize,
        rows: usize,
    },
    CursorMove {
        x: usize,
        y: usize,
    },
    CursorVisibility {
        visible: bool,
    },
    SetTitle {
        title: String,
    },
    FullSnapshot {
        snapshot: GridSnapshot,
    },
}

/// Full terminal grid snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridSnapshot {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Vec<Cell>>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub cursor_visible: bool,
    pub scroll_top: usize,
    pub scroll_bottom: usize,
    pub version: u64,
    pub title: String,
}

impl GridSnapshot {
    /// Create a new empty snapshot
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            cols,
            rows,
            cells: vec![vec![Cell::default(); cols]; rows],
            cursor_x: 0,
            cursor_y: 0,
            cursor_visible: true,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            version: 0,
            title: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_default() {
        let cell = Cell::default();
        assert_eq!(cell.char, ' ');
        assert_eq!(cell.fg, TerminalColor::Default);
        assert!(!cell.bold);
    }

    #[test]
    fn test_grid_snapshot_new() {
        let snapshot = GridSnapshot::new(80, 24);
        assert_eq!(snapshot.cols, 80);
        assert_eq!(snapshot.rows, 24);
        assert_eq!(snapshot.cells.len(), 24);
        assert_eq!(snapshot.cells[0].len(), 80);
    }

    #[test]
    fn test_grid_operation_serialization() {
        let op = GridOperation::CursorMove { x: 10, y: 5 };
        let json = serde_json::to_string(&op).unwrap();
        let deserialized: GridOperation = serde_json::from_str(&json).unwrap();

        match deserialized {
            GridOperation::CursorMove { x, y } => {
                assert_eq!(x, 10);
                assert_eq!(y, 5);
            }
            _ => panic!("Wrong operation type"),
        }
    }
}
