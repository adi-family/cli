# Prefer Small Crates

## Why This Rule Exists

Small, focused crates are easier to understand, test, and reuse. Rust's compilation unit is the crate -- splitting code enables parallel compilation. Dependencies can be shared across projects. The ecosystem thrives on composable single-purpose libraries.

The ADI crate structure (`core/` + `http/` + `plugin/`) follows this principle: business logic in core, transport layer separate, CLI integration separate. Each can be versioned, tested, and reasoned about independently.

## In Practice

- One crate = one responsibility
- Core logic in libraries, thin adapters for CLI/HTTP/plugin
- Use workspace members to share build artifacts and dependencies
- Extract reusable code into `crates/lib/lib-*` libraries
- Plugin functionality in `plugin/` that depends only on `core/`

## Trade-offs

**Benefits:**
- Parallel compilation
- Clear dependency graphs
- Reusable across projects
- Easier to understand and review

**Risks:**
- Version conflicts ("dependency hell")
- No automatic LTO across crates
- Need to vet third-party quality
- More boilerplate for cross-crate traits

## The Test

"Does this crate do more than one thing?" If yes, consider splitting it.

"Would this code be useful outside this project?" If yes, it should probably be a library crate.
