use std::time::Duration;

use tempfile::TempDir;

use crate::cache::RegistryCache;
use crate::error::RegistryError;
use adi_registry_core_cli::CliRegistryIndex;

fn test_index() -> CliRegistryIndex {
    CliRegistryIndex {
        version: 1,
        updated_at: 12345,
        plugins: vec![],
    }
}

fn cache_in(dir: &TempDir) -> RegistryCache {
    RegistryCache::new(dir.path().to_path_buf())
}

// ── 1. Cache paths ──

#[test]
fn cache_paths_are_correct() {
    let tmp = TempDir::new().unwrap();
    let cache = cache_in(&tmp);

    assert_eq!(cache.cache_dir(), tmp.path());
    assert_eq!(cache.index_path(), tmp.path().join("cli-registry").join("index.json"));
    assert_eq!(cache.downloads_dir(), tmp.path().join("downloads"));
    assert_eq!(
        cache.download_path("my-plugin", "1.0.0", "aarch64-apple-darwin"),
        tmp.path().join("downloads").join("my-plugin-1.0.0-aarch64-apple-darwin.tar.gz")
    );
}

// ── 2. Cache index roundtrip ──

#[test]
fn cache_index_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let cache = cache_in(&tmp);
    let index = test_index();

    cache.save_index(&index).unwrap();
    let loaded = cache.load_index().unwrap().expect("index should exist");

    assert_eq!(loaded.version, index.version);
    assert_eq!(loaded.updated_at, index.updated_at);
    assert!(loaded.plugins.is_empty());
}

// ── 3. Cache index TTL ──

#[test]
fn cache_index_ttl_valid_immediately() {
    let tmp = TempDir::new().unwrap();
    let cache = cache_in(&tmp);

    cache.save_index(&test_index()).unwrap();
    assert!(cache.is_index_valid());
}

#[test]
fn cache_index_ttl_zero_is_invalid() {
    let tmp = TempDir::new().unwrap();
    let cache = cache_in(&tmp).with_index_ttl(Duration::ZERO);

    cache.save_index(&test_index()).unwrap();
    // With zero TTL, the index is immediately expired
    assert!(!cache.is_index_valid());
}

// ── 4. Cache index missing ──

#[test]
fn cache_load_index_missing_returns_none() {
    let tmp = TempDir::new().unwrap();
    let cache = cache_in(&tmp);

    let result = cache.load_index().unwrap();
    assert!(result.is_none());
}

// ── 5. Cache download roundtrip ──

#[test]
fn cache_download_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let cache = cache_in(&tmp);
    let data = b"fake-plugin-binary-data";

    cache.save_download("my-plugin", "1.0.0", "linux-x86_64", data).unwrap();
    assert!(cache.has_download("my-plugin", "1.0.0", "linux-x86_64"));

    let loaded = cache.load_download("my-plugin", "1.0.0", "linux-x86_64").unwrap().expect("download should exist");
    assert_eq!(loaded, data);
}

// ── 6. Cache download missing ──

#[test]
fn cache_download_missing() {
    let tmp = TempDir::new().unwrap();
    let cache = cache_in(&tmp);

    assert!(!cache.has_download("nonexistent", "0.0.0", "any"));
    let result = cache.load_download("nonexistent", "0.0.0", "any").unwrap();
    assert!(result.is_none());
}

// ── 7. Cache clear all ──

#[test]
fn cache_clear_removes_everything() {
    let tmp = TempDir::new().unwrap();
    let cache = cache_in(&tmp);

    cache.save_index(&test_index()).unwrap();
    cache.save_download("p", "1.0", "linux", b"data").unwrap();
    assert!(cache.index_path().exists());
    assert!(cache.has_download("p", "1.0", "linux"));

    cache.clear().unwrap();

    assert!(!cache.index_path().exists());
    assert!(!cache.has_download("p", "1.0", "linux"));
}

// ── 8. Cache clear_index only ──

#[test]
fn cache_clear_index_keeps_downloads() {
    let tmp = TempDir::new().unwrap();
    let cache = cache_in(&tmp);

    cache.save_index(&test_index()).unwrap();
    cache.save_download("p", "1.0", "linux", b"data").unwrap();

    cache.clear_index().unwrap();

    assert!(!cache.index_path().exists());
    assert!(cache.has_download("p", "1.0", "linux"));
}

// ── 9. Cache clear_downloads only ──

#[test]
fn cache_clear_downloads_keeps_index() {
    let tmp = TempDir::new().unwrap();
    let cache = cache_in(&tmp);

    cache.save_index(&test_index()).unwrap();
    cache.save_download("p", "1.0", "linux", b"data").unwrap();

    cache.clear_downloads().unwrap();

    assert!(cache.index_path().exists());
    assert!(!cache.has_download("p", "1.0", "linux"));
}

// ── 10. Cache size ──

#[test]
fn cache_size_empty_is_zero() {
    let tmp = TempDir::new().unwrap();
    let cache = cache_in(&tmp);

    assert_eq!(cache.size(), 0);
}

#[test]
fn cache_size_with_data_is_positive() {
    let tmp = TempDir::new().unwrap();
    let cache = cache_in(&tmp);

    cache.save_index(&test_index()).unwrap();
    cache.save_download("p", "1.0", "linux", b"some binary content here").unwrap();

    assert!(cache.size() > 0);
}

// ── 11. Error display ──

#[test]
fn error_not_found_display() {
    let err = RegistryError::NotFound("foo".into());
    let msg = err.to_string();
    assert!(msg.contains("foo"), "expected 'foo' in: {msg}");
}

#[test]
fn error_version_not_found_display() {
    let err = RegistryError::VersionNotFound("my-plugin".into(), "2.0.0".into());
    let msg = err.to_string();
    assert!(msg.contains("my-plugin"), "expected plugin name in: {msg}");
    assert!(msg.contains("2.0.0"), "expected version in: {msg}");
}

#[test]
fn error_platform_not_supported_display() {
    let err = RegistryError::PlatformNotSupported("mips-unknown".into());
    let msg = err.to_string();
    assert!(msg.contains("mips-unknown"), "expected platform in: {msg}");
}

#[test]
fn error_invalid_response_display() {
    let err = RegistryError::InvalidResponse("bad json".into());
    let msg = err.to_string();
    assert!(msg.contains("bad json"), "expected message in: {msg}");
}

#[test]
fn error_cache_display() {
    let err = RegistryError::Cache("disk full".into());
    let msg = err.to_string();
    assert!(msg.contains("disk full"), "expected message in: {msg}");
}
