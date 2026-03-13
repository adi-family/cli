use std::path::Path;
use std::time::Instant;

use experiment_semantic_duplicates::scanner::{self, ScanConfig};
use lib_embed::{Embedder, FastEmbedder};

fn main() -> anyhow::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    println!("Scanning: {}", root.display());

    let start = Instant::now();
    let chunks = scanner::extract_chunks(root);
    println!("Extracted {} code chunks in {:.1}s", chunks.len(), start.elapsed().as_secs_f64());

    println!("Loading embedding model...");
    let emb = FastEmbedder::new()?;
    println!("Model loaded: {} ({}d)", emb.model_name(), emb.dimensions());

    let embed_start = Instant::now();
    let embeddings = scanner::embed_chunks(&emb, &chunks, 64)?;
    println!("Embedded {} chunks in {:.1}s", embeddings.len(), embed_start.elapsed().as_secs_f64());

    let config = ScanConfig {
        similarity_threshold: 0.85,
        cross_crate_only: true,
        max_results: 50,
    };

    let sim_start = Instant::now();
    let pairs = experiment_semantic_duplicates::similarity::find_similar_pairs(&embeddings, config.similarity_threshold);
    println!("Found {} pairs above {} in {:.1}s", pairs.len(), config.similarity_threshold, sim_start.elapsed().as_secs_f64());

    let duplicates: Vec<_> = pairs
        .into_iter()
        .filter(|pair| chunks[pair.idx_a].crate_name != chunks[pair.idx_b].crate_name)
        .take(config.max_results)
        .collect();

    println!("\n=== Top {} cross-crate semantic duplicates ===\n", duplicates.len());

    for (i, pair) in duplicates.iter().enumerate() {
        let a = &chunks[pair.idx_a];
        let b = &chunks[pair.idx_b];
        println!("#{} [{:.1}%]", i + 1, pair.similarity * 100.0);
        println!("  A: {} :: {}{} ({}:{})",
            a.crate_name, a.parent_type.as_deref().map(|p| format!("{p}::")).unwrap_or_default(),
            a.name, a.file_path.display(), a.line);
        println!("  B: {} :: {}{} ({}:{})",
            b.crate_name, b.parent_type.as_deref().map(|p| format!("{p}::")).unwrap_or_default(),
            b.name, b.file_path.display(), b.line);
        println!();
    }

    println!("Total time: {:.1}s", start.elapsed().as_secs_f64());
    Ok(())
}
