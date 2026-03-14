// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

pub mod analyzer;
pub mod config;
pub mod error;
pub mod graph;
pub mod indexer;
mod migrations;
pub mod parser;
pub mod search;
pub mod storage;
pub mod types;
pub mod watcher;

#[cfg(test)]
mod config_tests;

pub use analyzer::{
    AnalysisConfig, AnalysisMode, DeadCodeAnalyzer, DeadCodeFilter, DeadCodeReport,
    EntryPointDetector, ReachabilityAnalyzer, ReportFormat,
};
pub use config::Config;
pub use error::{Error, Result};
pub use graph::{
    calculate_metrics, detect_cycles, find_call_path, get_entry_points, get_leaf_nodes,
    get_transitive_callees, get_transitive_callers, get_usage_stats, SymbolMetrics,
};
pub use storage::sqlite::SqliteStorage;
pub use types::*;
pub use watcher::Watcher;

use crate::parser::Parser;
use crate::search::VectorIndex;
use crate::storage::Storage;
use lib_embed::PluginEmbedder;
use lib_plugin_host::PluginManagerV3;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;

// Re-export Embedder trait for plugin use
pub use lib_embed::Embedder;

pub struct Adi {
    project_path: PathBuf,
    config: Config,
    storage: Arc<dyn Storage>,
    embedder: Arc<dyn Embedder>,
    parser: Arc<dyn Parser>,
    index: Arc<dyn VectorIndex>,
}

impl Adi {
    /// Open an indexer with plugin support.
    /// Requires a PluginManagerV3 with language plugins (adi-lang-*) and adi.embed registered.
    ///
    /// Uses the adi.embed plugin for embeddings (much smaller binary).
    /// Install with: `adi plugin install adi.embed`
    pub async fn open_with_plugins(
        project_path: &Path,
        plugin_manager: Arc<PluginManagerV3>,
    ) -> Result<Self> {
        let config = Config::load(project_path)?;
        let adi_dir = project_path.join(".adi");

        std::fs::create_dir_all(&adi_dir)?;
        std::fs::create_dir_all(adi_dir.join("tree"))?;
        std::fs::create_dir_all(adi_dir.join("tree/embeddings"))?;
        std::fs::create_dir_all(adi_dir.join("cache"))?;

        let storage = SqliteStorage::open(&adi_dir.join("tree/index.sqlite"))?;
        let embedder = PluginEmbedder::new(plugin_manager.clone())?;
        let parser = parser::TreeSitterParser::new(plugin_manager);
        let index = search::usearch::UsearchIndex::open(&adi_dir.join("tree/embeddings"))?;

        Ok(Self {
            project_path: project_path.to_path_buf(),
            config,
            storage: Arc::new(storage),
            embedder: Arc::new(embedder),
            parser: Arc::new(parser),
            index: Arc::new(index),
        })
    }

    /// Open an indexer with a custom embedder.
    /// Use this when you have your own embedder implementation.
    pub async fn open_with_embedder(
        project_path: &Path,
        embedder: Arc<dyn Embedder>,
        plugin_manager: Arc<PluginManagerV3>,
    ) -> Result<Self> {
        let config = Config::load(project_path)?;
        let adi_dir = project_path.join(".adi");

        std::fs::create_dir_all(&adi_dir)?;
        std::fs::create_dir_all(adi_dir.join("tree"))?;
        std::fs::create_dir_all(adi_dir.join("tree/embeddings"))?;
        std::fs::create_dir_all(adi_dir.join("cache"))?;

        let storage = SqliteStorage::open(&adi_dir.join("tree/index.sqlite"))?;
        let parser = parser::TreeSitterParser::new(plugin_manager);
        let index = search::usearch::UsearchIndex::open(&adi_dir.join("tree/embeddings"))?;

        Ok(Self {
            project_path: project_path.to_path_buf(),
            config,
            storage: Arc::new(storage),
            embedder,
            parser: Arc::new(parser),
            index: Arc::new(index),
        })
    }

    /// Open an indexer without plugin support.
    /// Note: Parsing will fail for all languages without plugins installed.
    /// Use `open_with_plugins` for full functionality.
    ///
    /// Requires the `fastembed` feature on lib-embed for local embeddings.
    #[deprecated(note = "Use open_with_plugins instead for full functionality")]
    #[cfg(feature = "fastembed")]
    pub async fn open(project_path: &Path) -> Result<Self> {
        let config = Config::load(project_path)?;
        let adi_dir = project_path.join(".adi");

        std::fs::create_dir_all(&adi_dir)?;
        std::fs::create_dir_all(adi_dir.join("tree"))?;
        std::fs::create_dir_all(adi_dir.join("tree/embeddings"))?;
        std::fs::create_dir_all(adi_dir.join("cache"))?;

        let storage = SqliteStorage::open(&adi_dir.join("tree/index.sqlite"))?;
        let embedder = lib_embed::FastEmbedder::with_config(&config.embedding)?;
        let parser = parser::TreeSitterParser::without_plugins();
        let index = search::usearch::UsearchIndex::open(&adi_dir.join("tree/embeddings"))?;

        Ok(Self {
            project_path: project_path.to_path_buf(),
            config,
            storage: Arc::new(storage),
            embedder: Arc::new(embedder),
            parser: Arc::new(parser),
            index: Arc::new(index),
        })
    }

    pub fn project_path(&self) -> &Path {
        &self.project_path
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub async fn index(&self) -> Result<IndexProgress> {
        indexer::index_project(
            &self.project_path,
            &self.config,
            self.storage.clone(),
            self.embedder.clone(),
            self.parser.clone(),
            self.index.clone(),
        )
        .await
    }

    pub async fn reindex(&self, paths: &[PathBuf]) -> Result<()> {
        indexer::reindex_paths(
            &self.project_path,
            paths,
            &self.config,
            self.storage.clone(),
            self.embedder.clone(),
            self.parser.clone(),
            self.index.clone(),
        )
        .await
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        search::search(
            query,
            limit,
            self.storage.clone(),
            self.embedder.clone(),
            self.index.clone(),
        )
        .await
    }

    pub async fn search_symbols(&self, query: &str, limit: usize) -> Result<Vec<Symbol>> {
        search::search_symbols(query, limit, self.storage.clone()).await
    }

    pub async fn search_files(&self, query: &str, limit: usize) -> Result<Vec<File>> {
        search::search_files(query, limit, self.storage.clone()).await
    }

    pub fn get_file(&self, path: &Path) -> Result<FileInfo> {
        self.storage.get_file(path)
    }

    pub fn get_symbol(&self, id: SymbolId) -> Result<Symbol> {
        self.storage.get_symbol(id)
    }

    pub fn get_callers(&self, id: SymbolId) -> Result<Vec<Symbol>> {
        self.storage.get_callers(id)
    }

    pub fn get_callees(&self, id: SymbolId) -> Result<Vec<Symbol>> {
        self.storage.get_callees(id)
    }

    pub fn get_reference_count(&self, id: SymbolId) -> Result<u64> {
        self.storage.get_reference_count(id)
    }

    pub fn get_symbol_usage(&self, id: SymbolId) -> Result<SymbolUsage> {
        self.storage.get_symbol_usage(id)
    }

    pub fn find_symbols_by_name(&self, name: &str) -> Result<Vec<Symbol>> {
        self.storage.find_symbols_by_name(name)
    }

    pub fn get_references_to(&self, id: SymbolId) -> Result<Vec<Reference>> {
        self.storage.get_references_to(id)
    }

    pub fn get_references_from(&self, id: SymbolId) -> Result<Vec<Reference>> {
        self.storage.get_references_from(id)
    }

    pub fn get_tree(&self) -> Result<Tree> {
        self.storage.get_tree()
    }

    pub fn status(&self) -> Result<Status> {
        self.storage.get_status()
    }

    pub fn start_watching(&self) -> Result<mpsc::UnboundedReceiver<Vec<PathBuf>>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let watcher = Watcher::new(self.project_path.clone(), Arc::new(self.config.clone()), tx);
        watcher.start()?;
        Ok(rx)
    }
}
