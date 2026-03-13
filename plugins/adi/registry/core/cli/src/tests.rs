use crate::types::{CliPluginEntry, CliPluginInfo, CliRegistryIndex, CliSearchResults};
use crate::{validate_platform, CliRegistryStorage};

fn make_tar_gz(filename: &str, content: &[u8]) -> Vec<u8> {
    let mut archive = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    archive.append_data(&mut header, filename, content).unwrap();
    let tar_data = archive.into_inner().unwrap();

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    std::io::Write::write_all(&mut encoder, &tar_data).unwrap();
    encoder.finish().unwrap()
}

async fn publish_test_plugin(storage: &CliRegistryStorage, id: &str, version: &str, platform: &str) {
    let data = make_tar_gz("plugin.so", b"fake-plugin-binary");
    storage
        .publish_plugin(
            id,
            &format!("Test Plugin {id}"),
            "A test plugin",
            version,
            platform,
            &data,
            "test-author",
            vec!["test".to_string()],
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
}

// ---------------------------------------------------------------------------
// validate_platform
// ---------------------------------------------------------------------------

#[test]
fn validate_platform_empty_errors() {
    let result = validate_platform("");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must not be empty"));
}

#[test]
fn validate_platform_at_sign_errors() {
    assert!(validate_platform("linux@x86").is_err());
}

#[test]
fn validate_platform_dot_errors() {
    assert!(validate_platform("darwin.aarch64").is_err());
}

#[test]
fn validate_platform_space_errors() {
    assert!(validate_platform("linux x86_64").is_err());
}

#[test]
fn validate_platform_valid_hyphen() {
    assert!(validate_platform("darwin-aarch64").is_ok());
}

#[test]
fn validate_platform_valid_underscore() {
    assert!(validate_platform("linux_x86_64").is_ok());
}

#[test]
fn validate_platform_valid_alphanumeric() {
    assert!(validate_platform("win64").is_ok());
}

// ---------------------------------------------------------------------------
// types serde
// ---------------------------------------------------------------------------

#[test]
fn cli_plugin_entry_json_roundtrip() {
    let entry = CliPluginEntry {
        id: "my-plugin".into(),
        name: "My Plugin".into(),
        description: "desc".into(),
        plugin_types: vec!["cli".into()],
        latest_version: "1.0.0".into(),
        downloads: 42,
        author: "alice".into(),
        tags: vec!["util".into()],
    };
    let json = serde_json::to_string(&entry).unwrap();
    let deserialized: CliPluginEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, "my-plugin");
    assert_eq!(deserialized.name, "My Plugin");
    assert_eq!(deserialized.description, "desc");
    assert_eq!(deserialized.plugin_types, vec!["cli"]);
    assert_eq!(deserialized.latest_version, "1.0.0");
    assert_eq!(deserialized.downloads, 42);
    assert_eq!(deserialized.author, "alice");
    assert_eq!(deserialized.tags, vec!["util"]);
}

#[test]
fn cli_plugin_info_all_fields_roundtrip() {
    let info = CliPluginInfo {
        id: "plug".into(),
        version: "2.0.0".into(),
        platforms: vec![],
        published_at: 1700000000,
        changelog: Some("Fixed stuff".into()),
        preview_url: Some("https://example.com/preview".into()),
        preview_images: vec!["img1.png".into(), "img2.png".into()],
    };
    let json = serde_json::to_string(&info).unwrap();
    let deserialized: CliPluginInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, "plug");
    assert_eq!(deserialized.version, "2.0.0");
    assert_eq!(deserialized.published_at, 1700000000);
    assert_eq!(deserialized.changelog.as_deref(), Some("Fixed stuff"));
    assert_eq!(
        deserialized.preview_url.as_deref(),
        Some("https://example.com/preview")
    );
    assert_eq!(deserialized.preview_images, vec!["img1.png", "img2.png"]);
}

#[test]
fn cli_plugin_info_optional_fields_absent() {
    let json = r#"{"id":"x","version":"1.0.0","platforms":[]}"#;
    let info: CliPluginInfo = serde_json::from_str(json).unwrap();
    assert!(info.changelog.is_none());
    assert!(info.preview_url.is_none());
    assert!(info.preview_images.is_empty());
    assert_eq!(info.published_at, 0);
}

#[test]
fn cli_registry_index_default_values() {
    let index = CliRegistryIndex::default();
    assert_eq!(index.version, 1);
    assert_eq!(index.updated_at, 0);
    assert!(index.plugins.is_empty());
}

#[test]
fn cli_registry_index_json_roundtrip() {
    let mut index = CliRegistryIndex::default();
    index.plugins.push(CliPluginEntry {
        id: "test".into(),
        name: "Test".into(),
        description: String::new(),
        plugin_types: vec![],
        latest_version: "0.1.0".into(),
        downloads: 0,
        author: String::new(),
        tags: vec![],
    });
    let json = serde_json::to_string(&index).unwrap();
    let deserialized: CliRegistryIndex = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.version, 1);
    assert_eq!(deserialized.plugins.len(), 1);
    assert_eq!(deserialized.plugins[0].id, "test");
}

#[test]
fn cli_search_results_is_empty_when_empty() {
    let results = CliSearchResults::default();
    assert!(results.is_empty());
    assert_eq!(results.total(), 0);
}

#[test]
fn cli_search_results_total_counts() {
    let results = CliSearchResults {
        plugins: vec![
            CliPluginEntry {
                id: "a".into(),
                name: "A".into(),
                description: String::new(),
                plugin_types: vec![],
                latest_version: "1.0.0".into(),
                downloads: 0,
                author: String::new(),
                tags: vec![],
            },
            CliPluginEntry {
                id: "b".into(),
                name: "B".into(),
                description: String::new(),
                plugin_types: vec![],
                latest_version: "2.0.0".into(),
                downloads: 0,
                author: String::new(),
                tags: vec![],
            },
        ],
    };
    assert!(!results.is_empty());
    assert_eq!(results.total(), 2);
}

// ---------------------------------------------------------------------------
// CliRegistryStorage integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn storage_init_creates_index_with_defaults() {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage = CliRegistryStorage::new(tmp.path().to_path_buf());
    storage.init().await.unwrap();

    let index = storage.load_index().await.unwrap();
    assert_eq!(index.version, 1);
    assert_eq!(index.updated_at, 0);
    assert!(index.plugins.is_empty());
    assert!(tmp.path().join("index.json").exists());
}

#[tokio::test]
async fn storage_publish_writes_artifact_and_updates_index() {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage = CliRegistryStorage::new(tmp.path().to_path_buf());
    storage.init().await.unwrap();

    publish_test_plugin(&storage, "my-plugin", "1.0.0", "darwin-aarch64").await;

    let artifact = storage.artifact_path("my-plugin", "1.0.0", "darwin-aarch64");
    assert!(artifact.exists());

    let index = storage.load_index().await.unwrap();
    assert_eq!(index.plugins.len(), 1);
    assert_eq!(index.plugins[0].id, "my-plugin");
    assert_eq!(index.plugins[0].latest_version, "1.0.0");
    assert_ne!(index.updated_at, 0);
}

#[tokio::test]
async fn storage_get_plugin_info_returns_published() {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage = CliRegistryStorage::new(tmp.path().to_path_buf());
    storage.init().await.unwrap();

    publish_test_plugin(&storage, "info-test", "0.5.0", "linux-x86_64").await;

    let info = storage.get_plugin_info("info-test", "0.5.0").await.unwrap();
    assert_eq!(info.id, "info-test");
    assert_eq!(info.version, "0.5.0");
    assert_eq!(info.platforms.len(), 1);
    assert_eq!(info.platforms[0].platform, "linux-x86_64");
    assert!(info.platforms[0].size_bytes > 0);
    assert!(!info.platforms[0].checksum.is_empty());
}

#[tokio::test]
async fn storage_get_plugin_latest_returns_latest() {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage = CliRegistryStorage::new(tmp.path().to_path_buf());
    storage.init().await.unwrap();

    publish_test_plugin(&storage, "latest-test", "1.0.0", "darwin-aarch64").await;

    let info = storage.get_plugin_latest("latest-test").await.unwrap();
    assert_eq!(info.id, "latest-test");
    assert_eq!(info.version, "1.0.0");
}

#[tokio::test]
async fn storage_publish_newer_version_updates_latest() {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage = CliRegistryStorage::new(tmp.path().to_path_buf());
    storage.init().await.unwrap();

    publish_test_plugin(&storage, "semver-test", "1.0.0", "darwin-aarch64").await;
    publish_test_plugin(&storage, "semver-test", "2.0.0", "darwin-aarch64").await;

    let index = storage.load_index().await.unwrap();
    let entry = index.plugins.iter().find(|p| p.id == "semver-test").unwrap();
    assert_eq!(entry.latest_version, "2.0.0");
}

#[tokio::test]
async fn storage_publish_older_version_does_not_update_latest() {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage = CliRegistryStorage::new(tmp.path().to_path_buf());
    storage.init().await.unwrap();

    publish_test_plugin(&storage, "old-ver", "2.0.0", "darwin-aarch64").await;
    publish_test_plugin(&storage, "old-ver", "1.0.0", "darwin-aarch64").await;

    let index = storage.load_index().await.unwrap();
    let entry = index.plugins.iter().find(|p| p.id == "old-ver").unwrap();
    assert_eq!(entry.latest_version, "2.0.0");
}

#[tokio::test]
async fn storage_list_plugin_versions_sorted() {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage = CliRegistryStorage::new(tmp.path().to_path_buf());
    storage.init().await.unwrap();

    publish_test_plugin(&storage, "ver-list", "2.0.0", "darwin-aarch64").await;
    publish_test_plugin(&storage, "ver-list", "1.0.0", "darwin-aarch64").await;
    publish_test_plugin(&storage, "ver-list", "1.5.0", "darwin-aarch64").await;

    let versions = storage.list_plugin_versions("ver-list").await.unwrap();
    assert_eq!(versions, vec!["1.0.0", "1.5.0", "2.0.0"]);
}

#[tokio::test]
async fn storage_increment_downloads() {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage = CliRegistryStorage::new(tmp.path().to_path_buf());
    storage.init().await.unwrap();

    publish_test_plugin(&storage, "dl-test", "1.0.0", "darwin-aarch64").await;

    storage.increment_downloads("dl-test").await.unwrap();
    storage.increment_downloads("dl-test").await.unwrap();
    storage.increment_downloads("dl-test").await.unwrap();

    let index = storage.load_index().await.unwrap();
    let entry = index.plugins.iter().find(|p| p.id == "dl-test").unwrap();
    assert_eq!(entry.downloads, 3);
}

#[tokio::test]
async fn storage_publish_invalid_id_errors() {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage = CliRegistryStorage::new(tmp.path().to_path_buf());
    storage.init().await.unwrap();

    let data = make_tar_gz("plugin.so", b"data");
    let result = storage
        .publish_plugin(
            "invalid id!",
            "Name",
            "Desc",
            "1.0.0",
            "darwin-aarch64",
            &data,
            "author",
            vec![],
            None,
            None,
            None,
            None,
        )
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid ID"));
}

#[tokio::test]
async fn storage_publish_invalid_version_errors() {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage = CliRegistryStorage::new(tmp.path().to_path_buf());
    storage.init().await.unwrap();

    let data = make_tar_gz("plugin.so", b"data");
    let result = storage
        .publish_plugin(
            "valid-id",
            "Name",
            "Desc",
            "not-semver",
            "darwin-aarch64",
            &data,
            "author",
            vec![],
            None,
            None,
            None,
            None,
        )
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("semver"));
}

#[tokio::test]
async fn storage_publish_invalid_platform_errors() {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage = CliRegistryStorage::new(tmp.path().to_path_buf());
    storage.init().await.unwrap();

    let data = make_tar_gz("plugin.so", b"data");
    let result = storage
        .publish_plugin(
            "valid-id",
            "Name",
            "Desc",
            "1.0.0",
            "bad platform!",
            &data,
            "author",
            vec![],
            None,
            None,
            None,
            None,
        )
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid platform"));
}
