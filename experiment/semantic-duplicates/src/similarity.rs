/// Cosine similarity between two vectors. Returns value in [-1.0, 1.0].
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "vectors must have equal dimensions");

    let (dot, norm_a, norm_b) = a.iter().zip(b.iter()).fold(
        (0.0f64, 0.0f64, 0.0f64),
        |(dot, na, nb), (&x, &y)| {
            let (x, y) = (x as f64, y as f64);
            (dot + x * y, na + x * x, nb + y * y)
        },
    );

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < f64::EPSILON {
        return 0.0;
    }

    (dot / denom) as f32
}

/// A pair of indices with their similarity score.
#[derive(Debug, Clone)]
pub struct SimilarPair {
    pub idx_a: usize,
    pub idx_b: usize,
    pub similarity: f32,
}

/// Find all pairs above `threshold` from a set of embedding vectors.
/// Returns pairs sorted by similarity descending.
pub fn find_similar_pairs(embeddings: &[Vec<f32>], threshold: f32) -> Vec<SimilarPair> {
    let n = embeddings.len();
    let mut pairs = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            let sim = cosine_similarity(&embeddings[i], &embeddings[j]);
            if sim >= threshold {
                pairs.push(SimilarPair {
                    idx_a: i,
                    idx_b: j,
                    similarity: sim,
                });
            }
        }
    }

    pairs.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6, "identical vectors should have similarity ~1.0, got {sim}");
    }

    #[test]
    fn test_cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6, "orthogonal vectors should have similarity ~0.0, got {sim}");
    }

    #[test]
    fn test_cosine_opposite_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-6, "opposite vectors should have similarity ~-1.0, got {sim}");
    }

    #[test]
    fn test_cosine_zero_vector() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![0.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0, "zero vector should give similarity 0.0");
    }

    #[test]
    fn test_find_similar_pairs_basic() {
        let embeddings = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.99, 0.1, 0.0],  // very similar to [0]
            vec![0.0, 1.0, 0.0],   // orthogonal to [0]
        ];

        let pairs = find_similar_pairs(&embeddings, 0.9);

        assert_eq!(pairs.len(), 1, "should find exactly one pair above 0.9");
        assert_eq!(pairs[0].idx_a, 0);
        assert_eq!(pairs[0].idx_b, 1);
        assert!(pairs[0].similarity > 0.9);
    }

    #[test]
    fn test_find_similar_pairs_sorted_descending() {
        let embeddings = vec![
            vec![1.0, 0.0],
            vec![0.9, 0.1],   // somewhat similar
            vec![0.99, 0.01], // very similar
        ];

        let pairs = find_similar_pairs(&embeddings, 0.5);

        // Should be sorted by similarity descending
        for window in pairs.windows(2) {
            assert!(
                window[0].similarity >= window[1].similarity,
                "pairs should be sorted descending"
            );
        }
    }

    #[test]
    fn test_find_similar_pairs_empty() {
        let embeddings: Vec<Vec<f32>> = vec![];
        let pairs = find_similar_pairs(&embeddings, 0.5);
        assert!(pairs.is_empty());
    }
}
