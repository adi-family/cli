//! VTE terminal emulation with grid, parser, and PTY support
//!
//! Framework-agnostic terminal emulation library.

mod color;
mod grid;
mod parser;
mod pty;
mod terminal;

pub use color::{Color, NamedColor};
pub use grid::{Cell, CellAttributes, Grid};
pub use parser::Parser;
pub use pty::{create_shared_pty, PtyHandler, SharedPty};
pub use terminal::{create_shared_terminal, Selection, SharedTerminal, Terminal};
