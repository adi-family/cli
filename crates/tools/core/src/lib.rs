//! ADI Tools Core - Searchable CLI Tool Index
//!
//! Maintains a searchable index of available CLI tools with one-line descriptions.
//! LLM agents get a single meta-command to search tools by intent and pull full
//! usage docs only when needed.
//!
//! ## Tool Convention
//!
//! Tools are executables that follow this convention:
//! - `tool --help` - Human-readable usage (required)
//! - `tool --json` - Output JSON (optional)
//! - `tool describe` - One-line description (optional)
//!
//! ## Example
//!
//! ```rust,ignore
//! use tools_core::{ToolIndex, ToolSearch};
//!
//! let index = ToolIndex::open_global()?;
//! let results = index.find("list running containers", 10)?;
//! for result in results {
//!     println!("{}: {}", result.tool.name, result.tool.description);
//! }
//! ```

mod error;
mod types;
mod storage;
mod discovery;
mod search;
mod help_parser;

pub use error::{Error, Result};
pub use types::*;
pub use storage::Storage;
pub use discovery::*;
pub use search::ToolSearch;
pub use help_parser::parse_help_text;
