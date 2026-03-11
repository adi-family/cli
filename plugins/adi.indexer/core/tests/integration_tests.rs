use indexer_core::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn create_test_project() -> tempfile::TempDir {
    let dir = tempdir().unwrap();

    // Create a simple Rust project structure
    let src_dir = dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create main.rs
    fs::write(
        src_dir.join("main.rs"),
        r#"
//! Main entry point

/// Main function
fn main() {
    println!("Hello, world!");
    let config = Config::new();
    process_data(&config);
}

/// Process data using the given config
fn process_data(config: &Config) {
    // Process logic here
}

/// Configuration struct
pub struct Config {
    /// Name of the application
    pub name: String,
    /// Debug mode flag
    pub debug: bool,
}

impl Config {
    /// Create a new config with defaults
    pub fn new() -> Self {
        Config {
            name: String::from("app"),
            debug: false,
        }
    }

    /// Create a debug config
    pub fn debug() -> Self {
        Config {
            name: String::from("debug"),
            debug: true,
        }
    }
}
"#,
    )
    .unwrap();

    // Create lib.rs
    fs::write(
        src_dir.join("lib.rs"),
        r#"
//! Library module

pub mod utils;
pub mod handlers;

/// Re-export common types
pub use utils::*;
pub use handlers::*;
"#,
    )
    .unwrap();

    // Create utils module
    let utils_dir = src_dir.join("utils");
    fs::create_dir_all(&utils_dir).unwrap();

    fs::write(
        utils_dir.join("mod.rs"),
        r#"
//! Utility functions

/// Helper function to format strings
pub fn format_string(s: &str) -> String {
    s.trim().to_uppercase()
}

/// Parse a number from string
pub fn parse_number(s: &str) -> Option<i32> {
    s.parse().ok()
}

/// Math utilities
pub struct MathUtils;

impl MathUtils {
    /// Add two numbers
    pub fn add(a: i32, b: i32) -> i32 {
        a + b
    }

    /// Multiply two numbers
    pub fn multiply(a: i32, b: i32) -> i32 {
        a * b
    }
}
"#,
    )
    .unwrap();

    // Create handlers module
    let handlers_dir = src_dir.join("handlers");
    fs::create_dir_all(&handlers_dir).unwrap();

    fs::write(
        handlers_dir.join("mod.rs"),
        r#"
//! Request handlers

/// Handle HTTP requests
pub trait RequestHandler {
    fn handle(&self, request: &str) -> String;
}

/// User handler
pub struct UserHandler {
    name: String,
}

impl UserHandler {
    pub fn new(name: &str) -> Self {
        UserHandler {
            name: name.to_string(),
        }
    }
}

impl RequestHandler for UserHandler {
    fn handle(&self, request: &str) -> String {
        format!("User {} handling: {}", self.name, request)
    }
}
"#,
    )
    .unwrap();

    dir
}

fn create_python_project() -> tempfile::TempDir {
    let dir = tempdir().unwrap();

    fs::write(
        dir.path().join("main.py"),
        r#"
"""Main module for the application."""

class Application:
    """Main application class."""

    def __init__(self, name: str):
        """Initialize the application."""
        self.name = name

    def run(self):
        """Run the application."""
        print(f"Running {self.name}")

def main():
    """Entry point."""
    app = Application("MyApp")
    app.run()

if __name__ == "__main__":
    main()
"#,
    )
    .unwrap();

    fs::write(
        dir.path().join("utils.py"),
        r#"
"""Utility functions."""

def format_name(name: str) -> str:
    """Format a name properly."""
    return name.strip().title()

def calculate_total(items: list) -> float:
    """Calculate total of items."""
    return sum(items)

class Calculator:
    """Simple calculator class."""

    @staticmethod
    def add(a: float, b: float) -> float:
        """Add two numbers."""
        return a + b

    @staticmethod
    def subtract(a: float, b: float) -> float:
        """Subtract b from a."""
        return a - b
"#,
    )
    .unwrap();

    dir
}

#[test]
fn test_config_load_default() {
    let dir = tempdir().unwrap();
    let config = Config::load(dir.path()).unwrap();

    assert_eq!(config.embedding.provider, "fastembed");
    assert_eq!(config.embedding.dimensions, 768);
}

#[test]
fn test_config_load_with_project_config() {
    let dir = tempdir().unwrap();

    // Create .adi directory and config
    let adi_dir = dir.path().join(".adi");
    fs::create_dir_all(&adi_dir).unwrap();

    fs::write(
        adi_dir.join("config.toml"),
        r#"
[embedding]
dimensions = 512

[parser]
max_file_size = 2097152
"#,
    )
    .unwrap();

    let config = Config::load(dir.path()).unwrap();

    assert_eq!(config.embedding.dimensions, 512);
    assert_eq!(config.parser.max_file_size, 2097152);
}

#[test]
fn test_language_from_extension() {
    assert_eq!(Language::from_extension("rs"), Language::Rust);
    assert_eq!(Language::from_extension("py"), Language::Python);
    assert_eq!(Language::from_extension("js"), Language::JavaScript);
    assert_eq!(Language::from_extension("ts"), Language::TypeScript);
    assert_eq!(Language::from_extension("go"), Language::Go);
    assert_eq!(Language::from_extension("java"), Language::Java);
    assert_eq!(Language::from_extension("cpp"), Language::Cpp);
    assert_eq!(Language::from_extension("unknown_ext"), Language::Unknown);
}

#[test]
fn test_symbol_kind_conversions() {
    assert_eq!(SymbolKind::Function.as_str(), "function");
    assert_eq!(SymbolKind::Method.as_str(), "method");
    assert_eq!(SymbolKind::Class.as_str(), "class");
    assert_eq!(SymbolKind::Struct.as_str(), "struct");

    assert_eq!(SymbolKind::parse("function"), SymbolKind::Function);
    assert_eq!(SymbolKind::parse("class"), SymbolKind::Class);
    assert_eq!(SymbolKind::parse("invalid"), SymbolKind::Unknown);
}

#[test]
fn test_file_id_and_symbol_id() {
    let file_id = FileId(42);
    let symbol_id = SymbolId(123);

    assert_eq!(file_id.0, 42);
    assert_eq!(symbol_id.0, 123);
}

#[test]
fn test_location_struct() {
    let loc = Location {
        start_line: 10,
        start_col: 5,
        end_line: 20,
        end_col: 10,
        start_byte: 100,
        end_byte: 500,
    };

    assert_eq!(loc.start_line, 10);
    assert_eq!(loc.end_line, 20);
    assert!(loc.end_byte > loc.start_byte);
}

#[test]
fn test_index_progress_struct() {
    let progress = IndexProgress {
        files_processed: 10,
        files_total: 100,
        symbols_indexed: 50,
        errors: vec!["Error 1".to_string()],
    };

    assert_eq!(progress.files_processed, 10);
    assert_eq!(progress.files_total, 100);
    assert_eq!(progress.errors.len(), 1);
}

#[test]
fn test_status_struct() {
    let status = Status {
        indexed_files: 100,
        indexed_symbols: 500,
        embedding_dimensions: 768,
        embedding_model: "test-model".to_string(),
        last_indexed: Some("2024-01-01".to_string()),
        storage_size_bytes: 1024 * 1024,
    };

    assert_eq!(status.indexed_files, 100);
    assert_eq!(status.indexed_symbols, 500);
    assert!(status.last_indexed.is_some());
}

// Note: The following tests require fastembed model download
// They are marked as #[ignore] and can be run with: cargo test -- --ignored

#[tokio::test]
#[ignore]
async fn test_adi_open() {
    let project = create_test_project();
    let adi = Adi::open(project.path()).await.unwrap();

    assert_eq!(adi.project_path(), project.path());
}

#[tokio::test]
#[ignore]
async fn test_adi_index() {
    let project = create_test_project();
    let adi = Adi::open(project.path()).await.unwrap();

    let progress = adi.index().await.unwrap();

    assert!(progress.files_processed > 0);
    assert!(progress.symbols_indexed > 0);
}

#[tokio::test]
#[ignore]
async fn test_adi_status() {
    let project = create_test_project();
    let adi = Adi::open(project.path()).await.unwrap();

    // Index first
    adi.index().await.unwrap();

    let status = adi.status().unwrap();
    assert!(status.indexed_files > 0);
}

#[tokio::test]
#[ignore]
async fn test_adi_search() {
    let project = create_test_project();
    let adi = Adi::open(project.path()).await.unwrap();

    // Index first
    adi.index().await.unwrap();

    // Search for functions
    let _results = adi.search("process data", 10).await.unwrap();
    // Results may vary based on indexing
}

#[tokio::test]
#[ignore]
async fn test_adi_get_tree() {
    let project = create_test_project();
    let adi = Adi::open(project.path()).await.unwrap();

    // Index first
    adi.index().await.unwrap();

    let tree = adi.get_tree().unwrap();
    assert!(!tree.files.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_adi_get_file() {
    let project = create_test_project();
    let adi = Adi::open(project.path()).await.unwrap();

    // Index first
    adi.index().await.unwrap();

    let file_info = adi.get_file(&PathBuf::from("src/main.rs")).unwrap();
    assert_eq!(file_info.file.language, Language::Rust);
}

#[tokio::test]
#[ignore]
async fn test_adi_search_symbols() {
    let project = create_test_project();
    let adi = Adi::open(project.path()).await.unwrap();

    // Index first
    adi.index().await.unwrap();

    let _symbols = adi.search_symbols("Config", 10).await.unwrap();
    // Should find the Config struct
}

#[tokio::test]
#[ignore]
async fn test_python_project_indexing() {
    let project = create_python_project();
    let adi = Adi::open(project.path()).await.unwrap();

    let progress = adi.index().await.unwrap();
    assert!(progress.files_processed >= 2);
}
