# Semantic Duplicates Detector

Detects semantically similar code across distant modules using code embeddings.

## How it works

1. **Parse** — `syn` extracts all functions/methods from `.rs` files (skips <5 meaningful lines)
2. **Embed** — `jina-embeddings-v2-base-code` (768d, ONNX/fastembed) converts each function body to a vector
3. **Compare** — Brute-force cosine similarity across all pairs
4. **Filter** — Only report pairs from different crates (cross-crate duplicates)

## Run

```bash
# Full codebase scan (downloads ~80MB model on first run)
cargo run -p experiment-semantic-duplicates

# Unit tests (no model needed)
cargo test -p experiment-semantic-duplicates

# Integration tests (needs model)
cargo test -p experiment-semantic-duplicates --test real_embedder -- --ignored --nocapture
```

## Results (2026-03-13)

Scanned full ADI monorepo.

| Metric | Value |
|--------|-------|
| Code chunks extracted | 5,601 |
| Parse time | 3.1s |
| Embedding time (CPU) | 23 min |
| Similarity search time (brute-force N^2) | 2.3 min |
| Pairs above 0.85 threshold | 4,251 |
| Total time | ~25 min |

### Top findings (all at 100% similarity — literal copy-paste)

| # | Category | Crate A | Crate B | Functions |
|---|----------|---------|---------|-----------|
| 1 | Crypto | `llm-proxy-core` | `credentials-core` | `SecretManager::from_hex`, `decrypt`, tests |
| 2 | Plugin boilerplate | `mux-plugin`, `tasks-plugin`, ... | `auth-plugin`, `signaling-plugin`, ... | `init`, `run_command`, `get_runtime` |
| 3 | Indexer types (3x) | `lib-indexer-lang-abi` | `lib-plugin-abi-v3`, `indexer-core` | `SymbolKind`, `Visibility`, `ReferenceKind` — `as_str`/`parse` |
| 4 | API client builder | `lib-client-google-docs` | 12+ other `lib-client-*` crates | `ClientBuilder::new`, `auth` |
| 5 | WebRTC manager | `lib-webrtc-manager` | `cocoon-core` | `send_data`, `list_sessions`, `get_session_state` |
| 6 | Service events | `tasks-core` | `knowledgebase-core` | `broadcast_event` |
| 7 | AdiService types | `lib-adi-service` | `cocoon-core` | `AdiPluginCapabilities::default`, `AdiMethodInfo::default` |

### Verdict

Approach is **validated**. All top-50 results are real, actionable duplicates — zero false positives.

### Known limitations

- Embedding on CPU is slow (~23 min for 5.6k chunks). GPU or embedding cache would fix this.
- Brute-force N^2 similarity (15.7M comparisons). `usearch` index would reduce to ~1s.
- Top-50 saturated with 100% (syntactic) clones. The 85–95% range (semantic-only duplicates) needs separate reporting to surface novel insights.
- No clustering — same duplication group appears as N separate pairs instead of one group.

### Possible improvements

- Disk-cached embeddings (skip unchanged files)
- `usearch` vector index for fast ANN search
- Cluster results by duplication group
- Split report: exact clones vs semantic-only duplicates
- JSON output for CI integration
