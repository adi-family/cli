use indexer_core::*;
use std::fs;
use tempfile::tempdir;

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
