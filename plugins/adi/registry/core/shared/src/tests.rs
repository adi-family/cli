use crate::archive::{extract_file_from_tar_gz, extract_files_from_tar_gz, extract_manifest};
use crate::manifest::Manifest;
use crate::publisher::PublisherStore;
use crate::signing::RegistryKeyPair;
use crate::storage::FileStorage;
use crate::types::{PlatformBuild, PublishResponse, PublisherCertificate};
use crate::validation::{validate_id, validate_version};
use crate::{now_unix, semver_greater};

use std::io::Write;

fn make_tar_gz(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, data) in files {
        let mut header = tar::Header::new_gnu();
        header.set_path(name).unwrap();
        header.set_size(data.len() as u64);
        header.set_cksum();
        builder.append(&header, *data).unwrap();
    }
    let tar_data = builder.into_inner().unwrap();
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&tar_data).unwrap();
    encoder.finish().unwrap()
}

// ── validation ──

#[test]
fn validate_id_rejects_empty() {
    assert!(validate_id("").is_err());
}

#[test]
fn validate_id_rejects_at_sign() {
    assert!(validate_id("bad@id").is_err());
}

#[test]
fn validate_id_rejects_space() {
    assert!(validate_id("bad id").is_err());
}

#[test]
fn validate_id_rejects_slash() {
    assert!(validate_id("bad/id").is_err());
}

#[test]
fn validate_id_rejects_exclamation() {
    assert!(validate_id("bad!id").is_err());
}

#[test]
fn validate_id_accepts_dotted() {
    assert!(validate_id("adi.plugin").is_ok());
}

#[test]
fn validate_id_accepts_hyphens_underscores_digits() {
    assert!(validate_id("my-plugin_1").is_ok());
}

#[test]
fn validate_version_rejects_empty() {
    assert!(validate_version("").is_err());
}

#[test]
fn validate_version_rejects_partial() {
    assert!(validate_version("1.0").is_err());
}

#[test]
fn validate_version_rejects_alpha() {
    assert!(validate_version("abc").is_err());
}

#[test]
fn validate_version_accepts_basic() {
    assert!(validate_version("1.0.0").is_ok());
}

#[test]
fn validate_version_accepts_prerelease() {
    assert!(validate_version("1.2.3-beta.1").is_ok());
}

// ── archive ──

#[test]
fn extract_file_existing() {
    let data = make_tar_gz(&[("hello.txt", b"world")]);
    let result = extract_file_from_tar_gz(&data, "hello.txt");
    assert_eq!(result, Some(b"world".to_vec()));
}

#[test]
fn extract_file_missing() {
    let data = make_tar_gz(&[("hello.txt", b"world")]);
    assert!(extract_file_from_tar_gz(&data, "nope.txt").is_none());
}

#[test]
fn extract_file_garbage_bytes() {
    assert!(extract_file_from_tar_gz(b"not a tar gz", "anything").is_none());
}

#[test]
fn extract_manifest_valid() {
    let manifest_json = serde_json::json!({
        "id": "test-plugin",
        "version": "1.0.0",
        "name": "Test Plugin"
    });
    let json_bytes = serde_json::to_vec(&manifest_json).unwrap();
    let data = make_tar_gz(&[("manifest.json", &json_bytes)]);
    let manifest = extract_manifest(&data).unwrap();
    assert_eq!(manifest.id, "test-plugin");
    assert_eq!(manifest.version, "1.0.0");
    assert_eq!(manifest.name, "Test Plugin");
    assert!(manifest.description.is_empty());
    assert!(manifest.tags.is_empty());
}

#[test]
fn extract_files_finds_subset() {
    let data = make_tar_gz(&[
        ("a.txt", b"aaa"),
        ("b.txt", b"bbb"),
        ("c.txt", b"ccc"),
    ]);
    let results = extract_files_from_tar_gz(&data, &["a.txt", "c.txt", "missing.txt"]);
    let names: Vec<&str> = results.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"a.txt"));
    assert!(names.contains(&"c.txt"));
    assert_eq!(results.len(), 2);
}

// ── types serde ──

#[test]
fn platform_build_roundtrip_full() {
    let build = PlatformBuild {
        platform: "x86_64-linux".into(),
        download_url: "/v1/plugins/test/1.0.0/x86_64-linux.tar.gz".into(),
        size_bytes: 1024,
        checksum: "abc123".into(),
        publisher_signature: Some("sig".into()),
        publisher_public_key: Some("key".into()),
        registry_signature: Some("regsig".into()),
        publisher_id: Some("pub1".into()),
        publisher_certificate: Some(PublisherCertificate {
            publisher_id: "pub1".into(),
            publisher_public_key: "key".into(),
            registry_signature: "regsig".into(),
            created_at: 1000,
        }),
    };
    let json = serde_json::to_string(&build).unwrap();
    let decoded: PlatformBuild = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.platform, "x86_64-linux");
    assert_eq!(decoded.size_bytes, 1024);
    assert_eq!(decoded.publisher_id.as_deref(), Some("pub1"));
}

#[test]
fn platform_build_minimal_fields() {
    let json = serde_json::json!({
        "platform": "aarch64-macos",
        "downloadUrl": "/download",
        "checksum": "deadbeef"
    });
    let build: PlatformBuild = serde_json::from_value(json).unwrap();
    assert_eq!(build.platform, "aarch64-macos");
    assert_eq!(build.size_bytes, 0);
    assert!(build.publisher_signature.is_none());
    assert!(build.publisher_certificate.is_none());
}

#[test]
fn publisher_certificate_roundtrip() {
    let cert = PublisherCertificate {
        publisher_id: "pub1".into(),
        publisher_public_key: "pk".into(),
        registry_signature: "sig".into(),
        created_at: 12345,
    };
    let json = serde_json::to_string(&cert).unwrap();
    let decoded: PublisherCertificate = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.publisher_id, "pub1");
    assert_eq!(decoded.created_at, 12345);
}

#[test]
fn publish_response_roundtrip() {
    let resp = PublishResponse {
        status: "ok".into(),
        id: "my-plugin".into(),
        version: "1.0.0".into(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let decoded: PublishResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.status, "ok");
    assert_eq!(decoded.id, "my-plugin");
    assert_eq!(decoded.version, "1.0.0");
}

// ── manifest serde ──

#[test]
fn manifest_full_roundtrip() {
    let m = Manifest {
        id: "test".into(),
        version: "1.0.0".into(),
        name: "Test".into(),
        description: "A test".into(),
        author: "me".into(),
        tags: vec!["a".into(), "b".into()],
    };
    let json = serde_json::to_string(&m).unwrap();
    let decoded: Manifest = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.id, "test");
    assert_eq!(decoded.tags.len(), 2);
}

#[test]
fn manifest_defaults_for_optional_fields() {
    let json = serde_json::json!({
        "id": "x",
        "version": "0.1.0",
        "name": "X"
    });
    let m: Manifest = serde_json::from_value(json).unwrap();
    assert!(m.description.is_empty());
    assert!(m.author.is_empty());
    assert!(m.tags.is_empty());
}

// ── semver_greater ──

#[test]
fn semver_greater_higher_major() {
    assert!(semver_greater("2.0.0", "1.0.0"));
}

#[test]
fn semver_greater_lower_major() {
    assert!(!semver_greater("1.0.0", "2.0.0"));
}

#[test]
fn semver_greater_equal() {
    assert!(!semver_greater("1.0.0", "1.0.0"));
}

#[test]
fn semver_greater_invalid_falls_back_to_string() {
    // String comparison: "b" > "a"
    assert!(semver_greater("b", "a"));
    assert!(!semver_greater("a", "b"));
}

// ── now_unix ──

#[test]
fn now_unix_returns_nonzero() {
    assert!(now_unix() > 0);
}

// ── signing (async) ──

#[tokio::test]
async fn signing_ensure_exists_creates_files() {
    let dir = tempfile::tempdir().unwrap();
    let kp = RegistryKeyPair::new(dir.path().to_path_buf());
    kp.ensure_exists().await.unwrap();
    assert!(dir.path().join("registry_key.priv").exists());
    assert!(dir.path().join("registry_key.pub").exists());
}

#[tokio::test]
async fn signing_ensure_exists_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let kp = RegistryKeyPair::new(dir.path().to_path_buf());
    kp.ensure_exists().await.unwrap();
    let pub1 = kp.load_public_key().await.unwrap();
    kp.ensure_exists().await.unwrap();
    let pub2 = kp.load_public_key().await.unwrap();
    assert_eq!(pub1, pub2);
}

#[tokio::test]
async fn signing_load_public_key_nonempty() {
    let dir = tempfile::tempdir().unwrap();
    let kp = RegistryKeyPair::new(dir.path().to_path_buf());
    kp.ensure_exists().await.unwrap();
    let key = kp.load_public_key().await.unwrap();
    assert!(!key.is_empty());
}

#[tokio::test]
async fn signing_sign_returns_base64() {
    let dir = tempfile::tempdir().unwrap();
    let kp = RegistryKeyPair::new(dir.path().to_path_buf());
    kp.ensure_exists().await.unwrap();
    let sig = kp.sign(b"hello world").await.unwrap();
    assert!(!sig.is_empty());
    // Base64 chars: A-Z, a-z, 0-9, +, /, =
    assert!(sig.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '='));
}

// ── publisher (async) ──

#[tokio::test]
async fn publisher_init_creates_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = PublisherStore::new(dir.path().to_path_buf());
    store.init().await.unwrap();
    assert!(dir.path().join("publishers.json").exists());
}

#[tokio::test]
async fn publisher_register_creates_certificate() {
    let dir = tempfile::tempdir().unwrap();
    let kp = RegistryKeyPair::new(dir.path().to_path_buf());
    kp.ensure_exists().await.unwrap();
    let store = PublisherStore::new(dir.path().to_path_buf());
    store.init().await.unwrap();

    let cert = store.register(&kp, "pub1", "fake-key-123").await.unwrap();
    assert_eq!(cert.publisher_id, "pub1");
    assert_eq!(cert.publisher_public_key, "fake-key-123");
    assert!(!cert.registry_signature.is_empty());
}

#[tokio::test]
async fn publisher_register_same_key_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let kp = RegistryKeyPair::new(dir.path().to_path_buf());
    kp.ensure_exists().await.unwrap();
    let store = PublisherStore::new(dir.path().to_path_buf());
    store.init().await.unwrap();

    let cert1 = store.register(&kp, "pub1", "key-a").await.unwrap();
    let cert2 = store.register(&kp, "pub1", "key-a").await.unwrap();
    assert_eq!(cert1.registry_signature, cert2.registry_signature);
    assert_eq!(cert1.created_at, cert2.created_at);
}

#[tokio::test]
async fn publisher_register_different_key_errors() {
    let dir = tempfile::tempdir().unwrap();
    let kp = RegistryKeyPair::new(dir.path().to_path_buf());
    kp.ensure_exists().await.unwrap();
    let store = PublisherStore::new(dir.path().to_path_buf());
    store.init().await.unwrap();

    store.register(&kp, "pub1", "key-a").await.unwrap();
    let result = store.register(&kp, "pub1", "key-b").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("different key"));
}

#[tokio::test]
async fn publisher_revoke_makes_inactive() {
    let dir = tempfile::tempdir().unwrap();
    let kp = RegistryKeyPair::new(dir.path().to_path_buf());
    kp.ensure_exists().await.unwrap();
    let store = PublisherStore::new(dir.path().to_path_buf());
    store.init().await.unwrap();

    store.register(&kp, "pub1", "key-a").await.unwrap();
    store.register(&kp, "pub2", "key-b").await.unwrap();
    store.revoke("pub1").await.unwrap();

    let active = store.list_active().await.unwrap();
    let ids: Vec<&str> = active.iter().map(|p| p.publisher_id.as_str()).collect();
    assert!(!ids.contains(&"pub1"));
    assert!(ids.contains(&"pub2"));
}

#[tokio::test]
async fn publisher_list_active_excludes_revoked() {
    let dir = tempfile::tempdir().unwrap();
    let kp = RegistryKeyPair::new(dir.path().to_path_buf());
    kp.ensure_exists().await.unwrap();
    let store = PublisherStore::new(dir.path().to_path_buf());
    store.init().await.unwrap();

    store.register(&kp, "a", "k1").await.unwrap();
    store.register(&kp, "b", "k2").await.unwrap();
    store.register(&kp, "c", "k3").await.unwrap();
    store.revoke("b").await.unwrap();

    let active = store.list_active().await.unwrap();
    assert_eq!(active.len(), 2);
}

#[tokio::test]
async fn publisher_verify_certificate_valid() {
    let dir = tempfile::tempdir().unwrap();
    let kp = RegistryKeyPair::new(dir.path().to_path_buf());
    kp.ensure_exists().await.unwrap();
    let pub_key = kp.load_public_key().await.unwrap();
    let store = PublisherStore::new(dir.path().to_path_buf());
    store.init().await.unwrap();

    let cert = store.register(&kp, "pub1", &pub_key).await.unwrap();
    let valid = store.verify_certificate(&kp, &cert).await.unwrap();
    assert!(valid);
}

// ── storage (async) ──

#[tokio::test]
async fn storage_init_creates_dirs_and_files() {
    let dir = tempfile::tempdir().unwrap();
    let storage = FileStorage::new(dir.path().to_path_buf());
    storage.init(&["plugins", "tools"]).await.unwrap();
    assert!(dir.path().join("plugins").is_dir());
    assert!(dir.path().join("tools").is_dir());
    assert!(dir.path().join("publishers.json").exists());
    assert!(dir.path().join("registry_key.priv").exists());
    assert!(dir.path().join("registry_key.pub").exists());
}

#[tokio::test]
async fn storage_save_load_json_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let storage = FileStorage::new(dir.path().to_path_buf());
    storage.init(&[]).await.unwrap();

    let data = serde_json::json!({"name": "test", "count": 42});
    storage.save_json_atomic("test.json", &data).await.unwrap();
    let loaded: serde_json::Value = storage.load_json("test.json").await.unwrap();
    assert_eq!(loaded["name"], "test");
    assert_eq!(loaded["count"], 42);
}

#[tokio::test]
async fn storage_ensure_index_creates_if_missing() {
    let dir = tempfile::tempdir().unwrap();
    let storage = FileStorage::new(dir.path().to_path_buf());
    storage.init(&[]).await.unwrap();

    storage
        .ensure_index::<Vec<serde_json::Value>>("index.json")
        .await
        .unwrap();
    assert!(dir.path().join("index.json").exists());

    let loaded: Vec<serde_json::Value> = storage.load_json("index.json").await.unwrap();
    assert!(loaded.is_empty());
}

#[tokio::test]
async fn storage_ensure_index_preserves_existing() {
    let dir = tempfile::tempdir().unwrap();
    let storage = FileStorage::new(dir.path().to_path_buf());
    storage.init(&[]).await.unwrap();

    let data = vec![serde_json::json!({"id": "existing"})];
    storage.save_json_atomic("index.json", &data).await.unwrap();

    storage
        .ensure_index::<Vec<serde_json::Value>>("index.json")
        .await
        .unwrap();

    let loaded: Vec<serde_json::Value> = storage.load_json("index.json").await.unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0]["id"], "existing");
}

#[tokio::test]
async fn storage_list_artifact_versions_sorted() {
    let dir = tempfile::tempdir().unwrap();
    let storage = FileStorage::new(dir.path().to_path_buf());
    storage.init(&["plugins"]).await.unwrap();

    // Create version directories out of order
    let base = dir.path().join("plugins").join("my-plugin");
    tokio::fs::create_dir_all(base.join("2.0.0")).await.unwrap();
    tokio::fs::create_dir_all(base.join("1.0.0")).await.unwrap();
    tokio::fs::create_dir_all(base.join("1.5.0")).await.unwrap();
    // Non-semver dir should be ignored
    tokio::fs::create_dir_all(base.join("not-a-version"))
        .await
        .unwrap();

    let versions = storage
        .list_artifact_versions("plugins", "my-plugin")
        .await
        .unwrap();
    assert_eq!(versions, vec!["1.0.0", "1.5.0", "2.0.0"]);
}

#[tokio::test]
async fn storage_list_artifact_versions_empty_when_no_dir() {
    let dir = tempfile::tempdir().unwrap();
    let storage = FileStorage::new(dir.path().to_path_buf());
    storage.init(&["plugins"]).await.unwrap();

    let versions = storage
        .list_artifact_versions("plugins", "nonexistent")
        .await
        .unwrap();
    assert!(versions.is_empty());
}
