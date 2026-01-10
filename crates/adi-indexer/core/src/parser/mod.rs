// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

pub mod analyzer_registry;
pub mod grammar_registry;
pub mod plugin_adapter;
pub mod treesitter;

#[cfg(test)]
mod tests;

use crate::error::Result;
use crate::types::{Language, ParsedFile};

pub trait Parser: Send + Sync {
    fn parse(&self, source: &str, language: Language) -> Result<ParsedFile>;
    fn supports(&self, language: Language) -> bool;
}

// Re-export for convenience
pub use analyzer_registry::AnalyzerRegistry;
pub use grammar_registry::GrammarRegistry;
pub use treesitter::TreeSitterParser;
