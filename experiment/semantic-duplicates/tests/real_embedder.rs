/// Integration test using real fastembed model.
///
/// Downloads the jina-embeddings-v2-base-code model on first run (~80MB).
/// Run with: `cargo test -p experiment-semantic-duplicates --test real_embedder -- --ignored`
use experiment_semantic_duplicates::{
    scanner::{self, ScanConfig},
    similarity,
};
use lib_embed::{Embedder, FastEmbedder};

fn embedder() -> FastEmbedder {
    FastEmbedder::new().expect("failed to init fastembed - model may need downloading")
}

#[test]
#[ignore = "requires fastembed model download (~80MB)"]
fn real_embeddings_similar_code_scores_high() {
    let emb = embedder();

    let code_a = r#"
        fn read_config(path: &str) -> Config {
            let content = std::fs::read_to_string(path).unwrap();
            let config: Config = serde_json::from_str(&content).unwrap();
            config
        }
    "#;

    let code_b = r#"
        fn load_configuration(file_path: &str) -> Config {
            let data = std::fs::read_to_string(file_path).unwrap();
            let cfg: Config = serde_json::from_str(&data).unwrap();
            cfg
        }
    "#;

    let code_unrelated = r#"
        fn start_http_server(addr: &str) -> Server {
            let listener = TcpListener::bind(addr).unwrap();
            let server = Server::new(listener);
            server.run();
            server
        }
    "#;

    let embeddings = emb
        .embed(&[code_a, code_b, code_unrelated])
        .expect("embedding failed");

    let sim_ab = similarity::cosine_similarity(&embeddings[0], &embeddings[1]);
    let sim_ac = similarity::cosine_similarity(&embeddings[0], &embeddings[2]);

    println!("similarity(read_config, load_configuration) = {sim_ab:.4}");
    println!("similarity(read_config, start_http_server)  = {sim_ac:.4}");

    // Similar functions should score higher than unrelated ones
    assert!(
        sim_ab > sim_ac,
        "similar code ({sim_ab:.4}) should score higher than unrelated ({sim_ac:.4})"
    );

    // Similar functions should be above a reasonable threshold
    assert!(
        sim_ab > 0.75,
        "similar code should have cosine > 0.75, got {sim_ab:.4}"
    );
}

#[test]
#[ignore = "requires fastembed model download (~80MB)"]
fn real_scan_on_temp_crates() {
    let emb = embedder();
    let tmp = tempfile::TempDir::new().unwrap();

    // Create two fake crates with semantically similar functions
    for (crate_name, fn_name, body) in [
        (
            "crate-alpha",
            "fetch_user_data",
            r#"
fn fetch_user_data(user_id: &str) -> Result<User, Error> {
    let url = format!("https://api.example.com/users/{}", user_id);
    let response = reqwest::blocking::get(&url)?;
    let user: User = response.json()?;
    Ok(user)
}
"#,
        ),
        (
            "crate-beta",
            "get_user_info",
            r#"
fn get_user_info(uid: &str) -> Result<User, Error> {
    let endpoint = format!("https://api.example.com/users/{}", uid);
    let resp = reqwest::blocking::get(&endpoint)?;
    let user: User = resp.json()?;
    Ok(user)
}
"#,
        ),
        (
            "crate-gamma",
            "compress_archive",
            r#"
fn compress_archive(input_dir: &Path) -> Result<PathBuf, Error> {
    let output = input_dir.with_extension("tar.gz");
    let file = File::create(&output)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut archive = tar::Builder::new(encoder);
    archive.append_dir_all(".", input_dir)?;
    Ok(output)
}
"#,
        ),
    ] {
        let crate_dir = tmp.path().join(crate_name).join("src");
        std::fs::create_dir_all(&crate_dir).unwrap();
        std::fs::write(
            tmp.path().join(crate_name).join("Cargo.toml"),
            format!(
                "[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
            ),
        )
        .unwrap();
        std::fs::write(crate_dir.join("lib.rs"), body).unwrap();
        let _ = fn_name; // used in body
    }

    let config = ScanConfig {
        similarity_threshold: 0.75,
        cross_crate_only: true,
        max_results: 10,
    };

    let duplicates = scanner::scan(tmp.path(), &emb, &config).unwrap();

    println!("\n=== Detected duplicates ===");
    for dup in &duplicates {
        println!("{dup}");
    }

    // Should find the fetch_user_data <-> get_user_info pair
    let found_user_pair = duplicates.iter().any(|d| {
        let names = [&d.chunk_a.name, &d.chunk_b.name];
        names.contains(&&"fetch_user_data".to_string())
            && names.contains(&&"get_user_info".to_string())
    });

    assert!(
        found_user_pair,
        "should detect fetch_user_data and get_user_info as duplicates"
    );

    // compress_archive should NOT match the user functions
    let false_match = duplicates.iter().any(|d| {
        d.chunk_a.name == "compress_archive" || d.chunk_b.name == "compress_archive"
    });

    assert!(
        !false_match,
        "compress_archive should not match user-fetching functions"
    );
}

#[test]
#[ignore = "requires fastembed model download (~80MB), scans real codebase"]
fn real_scan_on_actual_codebase() {
    let emb = embedder();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

    let config = ScanConfig {
        similarity_threshold: 0.88,
        cross_crate_only: true,
        max_results: 30,
    };

    let chunks = scanner::extract_chunks(root);
    println!("Extracted {} code chunks from codebase", chunks.len());

    let duplicates = scanner::scan(root, &emb, &config).unwrap();

    println!("\n=== Top cross-crate semantic duplicates (threshold={}) ===", config.similarity_threshold);
    for (i, dup) in duplicates.iter().enumerate() {
        println!("\n--- #{} ---", i + 1);
        println!("{dup}");
        println!("  A: {}", dup.chunk_a.source.lines().next().unwrap_or(""));
        println!("  B: {}", dup.chunk_b.source.lines().next().unwrap_or(""));
    }

    println!("\nFound {} cross-crate duplicates above {}", duplicates.len(), config.similarity_threshold);
}
