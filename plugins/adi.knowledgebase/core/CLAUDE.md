adi-knowledgebase-core, rust, knowledge-management, graph-db, embeddings

## Overview
- Core library for ADI Knowledgebase - graph DB + embedding storage
- Dual storage: SQLite for graph, USearch for vector search
- Hybrid search: embedding similarity + graph traversal

## Node Types
- `Decision` - Architectural/product choices with rationale
- `Fact` - Immutable truths, definitions
- `Error` - Known issues with causes and fixes
- `Guide` - Procedural how-to knowledge
- `Glossary` - Term definitions
- `Context` - When/where knowledge applies
- `Assumption` - Unvalidated beliefs flagged for verification

## Edge Types
- `supersedes` - Version chain (new replaces old)
- `contradicts` - Conflict marker (requires resolution)
- `requires` - Dependency (A needs B)
- `related_to` - Weak association
- `derived_from` - Source reference
- `answers` - Maps questions to knowledge

## Confidence Levels
- 1.0 = Explicitly approved by user
- 0.8-0.99 = Strong evidence
- 0.5-0.79 = Reasonable inference
- 0.0-0.49 = Weak inference

## Dependencies
- `lib-embed` - Embedding generation
- `lib-migrations` - Database migrations
- `rusqlite` - Graph storage
- `usearch` - Vector search
