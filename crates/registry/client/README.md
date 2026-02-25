# lib-plugin-registry

Plugin registry HTTP client for the universal Rust plugin system.

## Overview

Async HTTP client for fetching plugins from a registry server. Supports caching, search, and progress-tracked downloads.

## Usage

```rust
use lib_plugin_registry::{RegistryClient, SearchKind};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RegistryClient::new("https://plugins.example.com")
        .with_cache(PathBuf::from("~/.cache/plugins"));

    // Fetch index
    let index = client.fetch_index().await?;
    println!("Found {} packages", index.packages.len());

    // Search
    let results = client.search("theme", SearchKind::All).await?;
    for pkg in results.packages {
        println!("{}: {}", pkg.id, pkg.name);
    }

    // Download with progress
    let bytes = client.download_package(
        "vendor.theme-pack",
        "1.0.0",
        "darwin-aarch64",
        |done, total| println!("Progress: {}/{}", done, total)
    ).await?;

    Ok(())
}
```

## Registry API

The client expects these endpoints:

```
GET /v1/index.json                               # Full index
GET /v1/packages/{id}/latest.json                # Latest package version
GET /v1/packages/{id}/{version}.json             # Specific version
GET /v1/packages/{id}/{version}/{platform}.tar.gz  # Download
GET /v1/search?q={query}&kind={kind}             # Search
```

## License

MIT
