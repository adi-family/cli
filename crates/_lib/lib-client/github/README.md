# lib-github-client

Async GitHub API client for Rust with pluggable authentication strategies.

## Features

- Async/await support via tokio
- Pluggable authentication (token, basic, custom)
- GitHub Enterprise support
- Typed responses for common GitHub resources
- Comprehensive error handling with rate limit detection

## Installation

```toml
[dependencies]
lib-github-client = { git = "https://github.com/adi-family/lib-github-client" }
```

## Quick Start

```rust
use lib_github_client::{Client, token};

#[tokio::main]
async fn main() -> lib_github_client::Result<()> {
    let client = Client::new(token("ghp_your_token"))?;

    let repo = client.get_repo("rust-lang", "rust").await?;
    println!("{}: {}", repo.full_name, repo.description.unwrap_or_default());

    Ok(())
}
```

## Authentication

### Token Authentication (recommended)

```rust
use lib_github_client::{Client, token};

let client = Client::new(token("ghp_xxx"))?;
```

### Basic Authentication

```rust
use lib_github_client::{Client, basic};

let client = Client::new(basic("username", "password"))?;
```

### Builder (for custom options)

```rust
use lib_github_client::{Client, token};

let client = Client::builder()
    .auth(token("ghp_xxx"))
    .base_url("https://github.mycompany.com/api/v3")  // GitHub Enterprise
    .user_agent("my-app/1.0")
    .build()?;
```

### Custom Authentication

Implement the `AuthStrategy` trait for custom auth mechanisms:

```rust
use lib_github_client::{AuthStrategy, Client, Result};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

struct GitHubAppAuth {
    jwt: String,
}

#[async_trait]
impl AuthStrategy for GitHubAppAuth {
    async fn apply(&self, headers: &mut HeaderMap) -> Result<()> {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.jwt)).unwrap(),
        );
        Ok(())
    }
}

let client = Client::new(GitHubAppAuth { jwt: "...".into() })?;
```

## API Reference

### Repository Operations

| Method | Description |
|--------|-------------|
| `get_repo(owner, repo)` | Get repository details |
| `list_branches(owner, repo)` | List all branches |
| `get_branch(owner, repo, branch)` | Get specific branch |

```rust
let repo = client.get_repo("owner", "repo").await?;
let branches = client.list_branches("owner", "repo").await?;
let main = client.get_branch("owner", "repo", "main").await?;
```

### Content Operations

| Method | Description |
|--------|-------------|
| `get_content(owner, repo, path, ref)` | Get file content |
| `create_or_update_file(...)` | Create or update a file |

```rust
// Get file from specific branch
let file = client.get_content("owner", "repo", "README.md", Some("main")).await?;

// Create new file
client.create_or_update_file(
    "owner", "repo", "path/to/file.txt",
    "Add new file",           // commit message
    "file contents here",     // content
    None,                     // sha (None for new file)
    Some("main"),             // branch
).await?;

// Update existing file
client.create_or_update_file(
    "owner", "repo", "path/to/file.txt",
    "Update file",
    "new contents",
    Some("abc123..."),        // existing file sha
    Some("main"),
).await?;
```

### Git Data Operations

| Method | Description |
|--------|-------------|
| `get_ref(owner, repo, ref_path)` | Get a git reference |
| `create_ref(owner, repo, ref_name, sha)` | Create a reference |
| `update_ref(owner, repo, ref_path, sha, force)` | Update a reference |
| `get_tree(owner, repo, tree_sha, recursive)` | Get a tree |
| `create_tree(owner, repo, base_tree, entries)` | Create a tree |
| `create_commit(owner, repo, message, tree_sha, parents)` | Create a commit |

```rust
// Get branch ref
let ref_info = client.get_ref("owner", "repo", "heads/main").await?;

// Create new branch
client.create_ref("owner", "repo", "refs/heads/feature", "sha123").await?;

// Force update branch
client.update_ref("owner", "repo", "heads/main", "newsha", true).await?;

// Get tree recursively
let tree = client.get_tree("owner", "repo", "sha123", true).await?;

// Create tree with new files
use lib_github_client::CreateTreeEntry;

let entries = vec![
    CreateTreeEntry {
        path: "file.txt".into(),
        mode: "100644".into(),
        content: Some("Hello World".into()),
        sha: None,
    },
];
let new_tree = client.create_tree("owner", "repo", Some("base_sha"), entries).await?;

// Create commit
let commit = client.create_commit(
    "owner", "repo",
    "Commit message",
    "tree_sha",
    vec!["parent_sha"],
).await?;
```

### User Operations

| Method | Description |
|--------|-------------|
| `get_authenticated_user()` | Get current authenticated user |

```rust
let user = client.get_authenticated_user().await?;
println!("Logged in as: {}", user.login);
```

### Release Operations

| Method | Description |
|--------|-------------|
| `list_releases(owner, repo)` | List all releases |
| `get_latest_release(owner, repo)` | Get latest release |
| `get_release_by_tag(owner, repo, tag)` | Get release by tag |
| `list_release_assets(owner, repo, release_id)` | List release assets |
| `download_asset(url)` | Download asset binary |

```rust
// Get latest release
let latest = client.get_latest_release("owner", "repo").await?;

// Get specific release
let release = client.get_release_by_tag("owner", "repo", "v1.0.0").await?;

// Download asset
let assets = client.list_release_assets("owner", "repo", release.id).await?;
let binary = client.download_asset(&assets[0].browser_download_url).await?;
```

## Types

### Repository

```rust
pub struct Repository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub private: bool,
    pub fork: bool,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub default_branch: String,
    pub stargazers_count: u64,
    pub forks_count: u64,
    pub open_issues_count: u64,
}
```

### Branch

```rust
pub struct Branch {
    pub name: String,
    pub commit: CommitRef,
    pub protected: bool,
}

pub struct CommitRef {
    pub sha: String,
    pub url: String,
}
```

### User

```rust
pub struct User {
    pub id: u64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: String,
    pub html_url: String,
}
```

### FileContent

```rust
pub struct FileContent {
    pub name: String,
    pub path: String,
    pub sha: String,
    pub size: u64,
    pub content: Option<String>,       // base64 encoded
    pub encoding: Option<String>,
    pub download_url: Option<String>,
}
```

### Tree

```rust
pub struct Tree {
    pub sha: String,
    pub tree: Vec<TreeEntry>,
    pub truncated: bool,
}

pub struct TreeEntry {
    pub path: String,
    pub mode: String,        // "100644" (file), "100755" (executable), "040000" (dir), "160000" (submodule)
    pub sha: String,
    pub entry_type: String,  // "blob", "tree", "commit"
    pub size: Option<u64>,
}

pub struct CreateTreeEntry {
    pub path: String,
    pub mode: String,
    pub content: Option<String>,  // for new files
    pub sha: Option<String>,      // for existing blobs
}
```

### Reference

```rust
pub struct Reference {
    pub ref_name: String,
    pub node_id: String,
    pub url: String,
    pub object: RefObject,
}

pub struct RefObject {
    pub sha: String,
    pub object_type: String,
    pub url: String,
}
```

### Release

```rust
pub struct Release {
    pub id: u64,
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub html_url: String,
    pub tarball_url: Option<String>,
    pub zipball_url: Option<String>,
}

pub struct ReleaseAsset {
    pub id: u64,
    pub name: String,
    pub content_type: String,
    pub size: u64,
    pub download_count: u64,
    pub browser_download_url: String,
}
```

## Error Handling

```rust
use lib_github_client::{GitHubError, Result};

match client.get_repo("owner", "repo").await {
    Ok(repo) => println!("Found: {}", repo.name),
    Err(GitHubError::NotFound(_)) => println!("Repository not found"),
    Err(GitHubError::Unauthorized) => println!("Invalid token"),
    Err(GitHubError::Forbidden) => println!("No access to repository"),
    Err(GitHubError::RateLimited { retry_after }) => {
        println!("Rate limited, retry after {} seconds", retry_after);
    }
    Err(GitHubError::Api { status, message }) => {
        println!("API error {}: {}", status, message);
    }
    Err(e) => println!("Error: {}", e),
}
```

### Error Types

| Error | Description |
|-------|-------------|
| `Request` | HTTP transport error |
| `Api { status, message }` | GitHub API error response |
| `RateLimited { retry_after }` | Rate limit exceeded |
| `NotFound` | Resource not found (404) |
| `Unauthorized` | Invalid or expired token (401) |
| `Forbidden` | Insufficient permissions (403) |
| `Json` | JSON parsing error |

## License

BSL-1.1
