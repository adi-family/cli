use crate::types::{WebPluginEntry, WebPluginInfo, WebRegistryIndex, WebSearchResults};
use crate::WebRegistryStorage;

fn make_tar_gz(files: &[(&str, &[u8])]) -> Vec<u8> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    {
        let mut builder = tar::Builder::new(&mut encoder);
        for (name, data) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, name, &data[..]).unwrap();
        }
        builder.finish().unwrap();
    }
    encoder.finish().unwrap()
}

fn make_manifest(id: &str, version: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "id": id,
        "version": version,
        "name": format!("Test Plugin {id}"),
        "description": "A test plugin",
        "author": "tester",
        "tags": ["test"]
    }))
    .unwrap()
}

async fn setup() -> (tempfile::TempDir, WebRegistryStorage) {
    let tmp = tempfile::TempDir::new().unwrap();
    let storage = WebRegistryStorage::new(tmp.path().to_path_buf());
    storage.init().await.unwrap();
    (tmp, storage)
}

// ── Type serde tests ──

#[test]
fn types_serde_web_plugin_entry_roundtrip() {
    let entry = WebPluginEntry {
        id: "my-plugin".into(),
        name: "My Plugin".into(),
        description: "desc".into(),
        latest_version: "1.0.0".into(),
        downloads: 42,
        author: "alice".into(),
        tags: vec!["ui".into(), "tool".into()],
    };
    let json = serde_json::to_string(&entry).unwrap();
    let back: WebPluginEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, entry.id);
    assert_eq!(back.name, entry.name);
    assert_eq!(back.description, entry.description);
    assert_eq!(back.latest_version, entry.latest_version);
    assert_eq!(back.downloads, entry.downloads);
    assert_eq!(back.author, entry.author);
    assert_eq!(back.tags, entry.tags);
}

#[test]
fn types_serde_web_plugin_info_with_optional_fields() {
    let info = WebPluginInfo {
        id: "test".into(),
        version: "1.0.0".into(),
        js_url: "/v1/test/1.0.0/main.js".into(),
        css_url: Some("/v1/test/1.0.0/main.css".into()),
        size_bytes: 1234,
        published_at: 100,
        changelog: None,
        preview_url: Some("/v1/test/1.0.0/preview.html".into()),
        preview_images: vec!["/v1/test/1.0.0/preview/0.webp".into()],
    };
    let json = serde_json::to_string(&info).unwrap();
    let back: WebPluginInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(back.css_url, info.css_url);
    assert_eq!(back.preview_url, info.preview_url);
    assert_eq!(back.preview_images, info.preview_images);
}

#[test]
fn types_serde_web_registry_index_default() {
    let idx = WebRegistryIndex::default();
    assert_eq!(idx.version, 1);
    assert_eq!(idx.updated_at, 0);
    assert!(idx.plugins.is_empty());

    let json = serde_json::to_string(&idx).unwrap();
    let back: WebRegistryIndex = serde_json::from_str(&json).unwrap();
    assert_eq!(back.version, 1);
}

#[test]
fn types_serde_web_search_results_default() {
    let results = WebSearchResults::default();
    assert!(results.plugins.is_empty());
}

// ── Storage tests ──

#[tokio::test]
async fn init_creates_index_json() {
    let (_tmp, storage) = setup().await;
    let index = storage.load_index().await.unwrap();
    assert_eq!(index.version, 1);
    assert!(index.plugins.is_empty());
}

#[tokio::test]
async fn publish_valid_archive_minimal() {
    let (_tmp, storage) = setup().await;
    let id = "my-plugin";
    let version = "1.0.0";
    let manifest = make_manifest(id, version);
    let js = b"console.log('hello');";

    let archive = make_tar_gz(&[("manifest.json", &manifest), ("main.js", js)]);
    storage.publish_web_plugin(id, version, &archive).await.unwrap();

    // main.js written
    let js_path = storage.js_path(id, version);
    let content = tokio::fs::read_to_string(&js_path).await.unwrap();
    assert_eq!(content, "console.log('hello');");

    // info.json created
    let info = storage.get_plugin_info(id, version).await.unwrap();
    assert_eq!(info.id, id);
    assert_eq!(info.version, version);
    assert_eq!(info.js_url, format!("/v1/{id}/{version}/main.js"));
    assert!(info.css_url.is_none());
    assert!(info.preview_url.is_none());
    assert_eq!(info.size_bytes, js.len() as u64);

    // index updated
    let index = storage.load_index().await.unwrap();
    assert_eq!(index.plugins.len(), 1);
    assert_eq!(index.plugins[0].id, id);
    assert_eq!(index.plugins[0].latest_version, version);
}

#[tokio::test]
async fn publish_with_css() {
    let (_tmp, storage) = setup().await;
    let id = "css-plugin";
    let version = "1.0.0";
    let manifest = make_manifest(id, version);
    let js = b"/* js */";
    let css = b"body { color: red; }";

    let archive = make_tar_gz(&[
        ("manifest.json", &manifest),
        ("main.js", js),
        ("main.css", css),
    ]);
    storage.publish_web_plugin(id, version, &archive).await.unwrap();

    let info = storage.get_plugin_info(id, version).await.unwrap();
    assert_eq!(info.css_url, Some(format!("/v1/{id}/{version}/main.css")));

    let css_content = tokio::fs::read_to_string(storage.css_path(id, version)).await.unwrap();
    assert_eq!(css_content, "body { color: red; }");
}

#[tokio::test]
async fn publish_with_preview_html() {
    let (_tmp, storage) = setup().await;
    let id = "preview-plugin";
    let version = "1.0.0";
    let manifest = make_manifest(id, version);
    let js = b"/* js */";
    let preview = b"<html><body>Preview</body></html>";

    let archive = make_tar_gz(&[
        ("manifest.json", &manifest),
        ("main.js", js),
        ("preview.html", preview),
    ]);
    storage.publish_web_plugin(id, version, &archive).await.unwrap();

    let info = storage.get_plugin_info(id, version).await.unwrap();
    assert_eq!(
        info.preview_url,
        Some(format!("/v1/{id}/{version}/preview.html"))
    );

    let preview_content =
        tokio::fs::read_to_string(storage.preview_html_path(id, version)).await.unwrap();
    assert_eq!(preview_content, "<html><body>Preview</body></html>");
}

#[tokio::test]
async fn publish_missing_manifest_errors() {
    let (_tmp, storage) = setup().await;
    let js = b"console.log('no manifest');";
    let archive = make_tar_gz(&[("main.js", js)]);

    let err = storage
        .publish_web_plugin("test", "1.0.0", &archive)
        .await
        .unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("manifest.json"),
        "expected error about manifest.json, got: {msg}"
    );
}

#[tokio::test]
async fn publish_missing_main_js_errors() {
    let (_tmp, storage) = setup().await;
    let manifest = make_manifest("test", "1.0.0");
    let archive = make_tar_gz(&[("manifest.json", &manifest)]);

    let err = storage
        .publish_web_plugin("test", "1.0.0", &archive)
        .await
        .unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.to_lowercase().contains("main.js"),
        "expected error about main.js, got: {msg}"
    );
}

#[tokio::test]
async fn publish_id_mismatch_errors() {
    let (_tmp, storage) = setup().await;
    let manifest = make_manifest("wrong-id", "1.0.0");
    let js = b"/* js */";
    let archive = make_tar_gz(&[("manifest.json", &manifest), ("main.js", js)]);

    let err = storage
        .publish_web_plugin("correct-id", "1.0.0", &archive)
        .await
        .unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("wrong-id") && msg.contains("correct-id"),
        "expected ID mismatch error, got: {msg}"
    );
}

#[tokio::test]
async fn publish_version_mismatch_errors() {
    let (_tmp, storage) = setup().await;
    let manifest = make_manifest("test", "2.0.0");
    let js = b"/* js */";
    let archive = make_tar_gz(&[("manifest.json", &manifest), ("main.js", js)]);

    let err = storage
        .publish_web_plugin("test", "1.0.0", &archive)
        .await
        .unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("2.0.0") && msg.contains("1.0.0"),
        "expected version mismatch error, got: {msg}"
    );
}

#[tokio::test]
async fn publish_newer_version_updates_latest() {
    let (_tmp, storage) = setup().await;
    let id = "evolving-plugin";

    let manifest_v1 = make_manifest(id, "1.0.0");
    let manifest_v2 = make_manifest(id, "2.0.0");
    let js = b"/* js */";

    let archive_v1 = make_tar_gz(&[("manifest.json", &manifest_v1), ("main.js", js)]);
    storage.publish_web_plugin(id, "1.0.0", &archive_v1).await.unwrap();

    let archive_v2 = make_tar_gz(&[("manifest.json", &manifest_v2), ("main.js", js)]);
    storage.publish_web_plugin(id, "2.0.0", &archive_v2).await.unwrap();

    let index = storage.load_index().await.unwrap();
    let entry = index.plugins.iter().find(|p| p.id == id).unwrap();
    assert_eq!(entry.latest_version, "2.0.0");
}

#[tokio::test]
async fn publish_older_version_does_not_update_latest() {
    let (_tmp, storage) = setup().await;
    let id = "stable-plugin";

    let manifest_v2 = make_manifest(id, "2.0.0");
    let manifest_v1 = make_manifest(id, "1.0.0");
    let js = b"/* js */";

    let archive_v2 = make_tar_gz(&[("manifest.json", &manifest_v2), ("main.js", js)]);
    storage.publish_web_plugin(id, "2.0.0", &archive_v2).await.unwrap();

    let archive_v1 = make_tar_gz(&[("manifest.json", &manifest_v1), ("main.js", js)]);
    storage.publish_web_plugin(id, "1.0.0", &archive_v1).await.unwrap();

    let index = storage.load_index().await.unwrap();
    let entry = index.plugins.iter().find(|p| p.id == id).unwrap();
    assert_eq!(entry.latest_version, "2.0.0");
}

#[tokio::test]
async fn increment_downloads_increases_count() {
    let (_tmp, storage) = setup().await;
    let id = "dl-plugin";
    let version = "1.0.0";
    let manifest = make_manifest(id, version);
    let js = b"/* js */";

    let archive = make_tar_gz(&[("manifest.json", &manifest), ("main.js", js)]);
    storage.publish_web_plugin(id, version, &archive).await.unwrap();

    storage.increment_downloads(id).await.unwrap();
    storage.increment_downloads(id).await.unwrap();
    storage.increment_downloads(id).await.unwrap();

    let index = storage.load_index().await.unwrap();
    let entry = index.plugins.iter().find(|p| p.id == id).unwrap();
    assert_eq!(entry.downloads, 3);
}

#[tokio::test]
async fn get_plugin_latest_returns_correct_info() {
    let (_tmp, storage) = setup().await;
    let id = "latest-plugin";
    let version = "1.0.0";
    let manifest = make_manifest(id, version);
    let js = b"latest js content";

    let archive = make_tar_gz(&[("manifest.json", &manifest), ("main.js", js)]);
    storage.publish_web_plugin(id, version, &archive).await.unwrap();

    let info = storage.get_plugin_latest(id).await.unwrap();
    assert_eq!(info.id, id);
    assert_eq!(info.version, version);
    assert_eq!(info.js_url, format!("/v1/{id}/{version}/main.js"));
    assert_eq!(info.size_bytes, js.len() as u64);
}
