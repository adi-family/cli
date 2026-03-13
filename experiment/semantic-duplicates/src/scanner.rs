use std::path::Path;

use lib_embed::Embedder;
use walkdir::WalkDir;

use crate::parser::{self, CodeChunk};
use crate::similarity::{self, SimilarPair};

/// A detected semantic duplicate across different crates/modules.
#[derive(Debug, Clone)]
pub struct SemanticDuplicate {
    pub chunk_a: CodeChunk,
    pub chunk_b: CodeChunk,
    pub similarity: f32,
}

impl std::fmt::Display for SemanticDuplicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label_a = format_chunk_label(&self.chunk_a);
        let label_b = format_chunk_label(&self.chunk_b);
        write!(
            f,
            "[{:.1}%] {} <-> {}",
            self.similarity * 100.0,
            label_a,
            label_b,
        )
    }
}

fn format_chunk_label(chunk: &CodeChunk) -> String {
    let fn_name = match &chunk.parent_type {
        Some(parent) => format!("{parent}::{}", chunk.name),
        None => chunk.name.clone(),
    };
    format!(
        "{}:{} ({})",
        chunk.file_path.display(),
        chunk.line,
        fn_name,
    )
}

/// Configuration for the scanner.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// Minimum cosine similarity to consider a pair (0.0 - 1.0).
    pub similarity_threshold: f32,
    /// Only report pairs from different crates.
    pub cross_crate_only: bool,
    /// Maximum results to return.
    pub max_results: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.82,
            cross_crate_only: true,
            max_results: 50,
        }
    }
}

/// Collect all Rust source files under a directory.
fn collect_rust_files(root: &Path) -> Vec<std::path::PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip build artifacts, hidden dirs, node_modules
            !matches!(name.as_ref(), "target" | ".git" | "node_modules" | "vendor")
        })
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().is_some_and(|ext| ext == "rs")
                && !e.path().to_string_lossy().contains("/target/")
        })
        .map(|e| e.into_path())
        .collect()
}

/// Parse all Rust files in a directory tree into code chunks.
pub fn extract_chunks(root: &Path) -> Vec<CodeChunk> {
    let files = collect_rust_files(root);
    let mut all_chunks = Vec::new();

    for file in &files {
        let crate_name = parser::find_crate_name(file)
            .unwrap_or_else(|| "unknown".into());

        match parser::parse_file(file, &crate_name) {
            Ok(chunks) => all_chunks.extend(chunks),
            Err(e) => {
                eprintln!("warning: failed to parse {}: {e}", file.display());
            }
        }
    }

    all_chunks
}

/// Embed all chunks using the provided embedder.
pub fn embed_chunks(
    embedder: &dyn Embedder,
    chunks: &[CodeChunk],
    batch_size: usize,
) -> anyhow::Result<Vec<Vec<f32>>> {
    let mut all_embeddings = Vec::with_capacity(chunks.len());

    for batch in chunks.chunks(batch_size) {
        let texts: Vec<&str> = batch.iter().map(|c| c.source.as_str()).collect();
        let batch_embeddings = embedder.embed(&texts)?;
        all_embeddings.extend(batch_embeddings);
    }

    Ok(all_embeddings)
}

/// Run the full scan pipeline: parse -> embed -> find cross-module duplicates.
pub fn scan(
    root: &Path,
    embedder: &dyn Embedder,
    config: &ScanConfig,
) -> anyhow::Result<Vec<SemanticDuplicate>> {
    let chunks = extract_chunks(root);
    if chunks.is_empty() {
        return Ok(Vec::new());
    }

    let embeddings = embed_chunks(embedder, &chunks, 64)?;
    let pairs = similarity::find_similar_pairs(&embeddings, config.similarity_threshold);

    let duplicates: Vec<SemanticDuplicate> = pairs
        .into_iter()
        .filter(|pair| {
            if config.cross_crate_only {
                chunks[pair.idx_a].crate_name != chunks[pair.idx_b].crate_name
            } else {
                // At least different files
                chunks[pair.idx_a].file_path != chunks[pair.idx_b].file_path
            }
        })
        .take(config.max_results)
        .map(|SimilarPair { idx_a, idx_b, similarity }| SemanticDuplicate {
            chunk_a: chunks[idx_a].clone(),
            chunk_b: chunks[idx_b].clone(),
            similarity,
        })
        .collect();

    Ok(duplicates)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeEmbedder;

    impl Embedder for FakeEmbedder {
        fn embed(&self, texts: &[&str]) -> lib_embed::Result<Vec<Vec<f32>>> {
            // Deterministic fake: hash text to produce a pseudo-embedding.
            // Similar texts will produce similar vectors because we use
            // character frequency as the embedding.
            Ok(texts.iter().map(|t| text_to_fake_embedding(t)).collect())
        }

        fn dimensions(&self) -> u32 {
            26
        }

        fn model_name(&self) -> &str {
            "fake-char-frequency"
        }
    }

    /// Convert text to a 26-dim vector of letter frequencies (a-z).
    fn text_to_fake_embedding(text: &str) -> Vec<f32> {
        let mut freq = vec![0.0f32; 26];
        let total = text.len().max(1) as f32;
        for ch in text.chars().flat_map(|c| c.to_lowercase()) {
            if ch.is_ascii_lowercase() {
                freq[(ch as u8 - b'a') as usize] += 1.0;
            }
        }
        // Normalize
        for f in &mut freq {
            *f /= total;
        }
        freq
    }

    #[test]
    fn test_collect_rust_files() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let files = collect_rust_files(&root);
        assert!(
            files.iter().any(|f| f.ends_with("lib.rs")),
            "should find lib.rs in src/"
        );
    }

    #[test]
    fn test_extract_chunks_from_self() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let chunks = extract_chunks(&root);
        assert!(!chunks.is_empty(), "should extract some chunks from our own source");
    }

    #[test]
    fn test_embed_chunks_with_fake() {
        let chunks = vec![
            CodeChunk {
                crate_name: "a".into(),
                file_path: "a.rs".into(),
                name: "foo".into(),
                source: "let x = read_file(path); process(x); write_output(result);".into(),
                line: 1,
                parent_type: None,
            },
            CodeChunk {
                crate_name: "b".into(),
                file_path: "b.rs".into(),
                name: "bar".into(),
                source: "let x = read_file(path); process(x); write_output(result);".into(),
                line: 1,
                parent_type: None,
            },
        ];

        let embedder = FakeEmbedder;
        let embeddings = embed_chunks(&embedder, &chunks, 32).unwrap();
        assert_eq!(embeddings.len(), 2);

        // Identical source => identical embeddings => similarity = 1.0
        let sim = crate::similarity::cosine_similarity(&embeddings[0], &embeddings[1]);
        assert!((sim - 1.0).abs() < 1e-6, "identical text should give sim=1.0");
    }

    #[test]
    fn test_scan_finds_cross_crate_duplicates() {
        // Create temp dir with two "crates" containing similar functions
        let tmp = tempfile::TempDir::new().unwrap();
        let crate_a = tmp.path().join("crate_a/src");
        let crate_b = tmp.path().join("crate_b/src");
        std::fs::create_dir_all(&crate_a).unwrap();
        std::fs::create_dir_all(&crate_b).unwrap();

        // Cargo.toml files for crate name resolution
        std::fs::write(
            tmp.path().join("crate_a/Cargo.toml"),
            "[package]\nname = \"crate-a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("crate_b/Cargo.toml"),
            "[package]\nname = \"crate-b\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();

        // Similar functions in different crates
        std::fs::write(
            crate_a.join("lib.rs"),
            r#"
fn read_and_process_file() {
    let path = get_config_path();
    let content = std::fs::read_to_string(path).unwrap();
    let parsed = serde_json::from_str(&content).unwrap();
    let result = transform(parsed);
    save_output(result);
}
"#,
        )
        .unwrap();

        std::fs::write(
            crate_b.join("lib.rs"),
            r#"
fn load_and_transform_file() {
    let path = get_config_path();
    let content = std::fs::read_to_string(path).unwrap();
    let parsed = serde_json::from_str(&content).unwrap();
    let result = transform(parsed);
    save_output(result);
}
"#,
        )
        .unwrap();

        let config = ScanConfig {
            similarity_threshold: 0.8,
            cross_crate_only: true,
            max_results: 10,
        };

        let duplicates = scan(tmp.path(), &FakeEmbedder, &config).unwrap();

        assert!(
            !duplicates.is_empty(),
            "should detect the cross-crate duplicate"
        );

        let dup = &duplicates[0];
        assert_ne!(dup.chunk_a.crate_name, dup.chunk_b.crate_name);
        assert!(dup.similarity > 0.8);
    }

    #[test]
    fn test_scan_ignores_same_crate_when_configured() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();

        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"same-crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();

        // Two similar functions in the SAME crate
        std::fs::write(
            src.join("a.rs"),
            r#"
fn do_something() {
    let path = get_path();
    let data = read_data(path);
    let result = process(data);
    write_result(result);
    log_completion();
}
"#,
        )
        .unwrap();

        std::fs::write(
            src.join("b.rs"),
            r#"
fn do_similar_thing() {
    let path = get_path();
    let data = read_data(path);
    let result = process(data);
    write_result(result);
    log_completion();
}
"#,
        )
        .unwrap();

        let config = ScanConfig {
            similarity_threshold: 0.8,
            cross_crate_only: true,
            max_results: 10,
        };

        let duplicates = scan(tmp.path(), &FakeEmbedder, &config).unwrap();
        assert!(
            duplicates.is_empty(),
            "should NOT report same-crate duplicates when cross_crate_only=true"
        );

        // But should find them when cross_crate_only=false
        let config_relaxed = ScanConfig {
            cross_crate_only: false,
            ..config
        };
        let duplicates = scan(tmp.path(), &FakeEmbedder, &config_relaxed).unwrap();
        assert!(
            !duplicates.is_empty(),
            "should find same-crate duplicates when cross_crate_only=false"
        );
    }
}
