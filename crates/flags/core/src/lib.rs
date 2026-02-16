use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("config parse: {0}")]
    Config(#[from] toml::de::Error),
    #[error("state '{0}' not configured in .adi/flags.toml")]
    UnknownState(String),
    #[error("no .adi/flags.toml found — run `adi flags init`")]
    NoConfig,
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Check mode for dirty detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckMode {
    /// mtime only — fast, may false-trigger
    Fast,
    /// mtime first, then hash to confirm — no false triggers
    Strict,
}

impl Default for CheckMode {
    fn default() -> Self {
        Self::Fast
    }
}

/// State definition from config
#[derive(Debug, Clone, serde::Deserialize)]
pub struct StateConfig {
    pub description: String,
}

/// Top-level config parsed from `.adi/flags.toml`
#[derive(Debug, serde::Deserialize)]
pub struct FlagsConfig {
    #[serde(default)]
    pub check: CheckMode,
    #[serde(default)]
    pub states: BTreeMap<String, StateConfig>,
}

/// One entry in the flag index file
#[derive(Debug, Clone)]
pub struct FlagEntry {
    pub path: String,
    pub mtime: u64,
    pub hash: String,
}

/// Status of a flagged file
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Clean,
    Dirty,
    Missing,
}

const CONFIG_PATH: &str = ".adi/flags.toml";
const CACHE_DIR: &str = ".adi/cache/flags";

pub fn config_path(root: &Path) -> PathBuf {
    root.join(CONFIG_PATH)
}

fn cache_dir(root: &Path) -> PathBuf {
    root.join(CACHE_DIR)
}

fn index_path(root: &Path, state: &str) -> PathBuf {
    cache_dir(root).join(state)
}

/// Load config from `.adi/flags.toml`
pub fn load_config(root: &Path) -> Result<FlagsConfig> {
    let path = config_path(root);
    if !path.exists() {
        return Err(Error::NoConfig);
    }
    let content = fs::read_to_string(&path)?;
    Ok(toml::from_str(&content)?)
}

/// Validate that a state exists in config
pub fn validate_state(config: &FlagsConfig, state: &str) -> Result<()> {
    if !config.states.contains_key(state) {
        return Err(Error::UnknownState(state.to_string()));
    }
    Ok(())
}

/// Load flag index for a given state
pub fn load_index(root: &Path, state: &str) -> Result<BTreeMap<String, FlagEntry>> {
    let path = index_path(root, state);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let file = fs::File::open(&path)?;
    let reader = BufReader::new(file);
    let mut entries = BTreeMap::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(entry) = parse_entry(line) {
            entries.insert(entry.path.clone(), entry);
        }
    }
    Ok(entries)
}

fn parse_entry(line: &str) -> Option<FlagEntry> {
    // Format: <path> <mtime> <hash>
    // Find last two space-separated tokens (hash and mtime), rest is path
    let parts: Vec<&str> = line.rsplitn(3, ' ').collect();
    if parts.len() != 3 {
        return None;
    }
    let hash = parts[0].to_string();
    let mtime = parts[1].parse::<u64>().ok()?;
    let path = parts[2].to_string();
    Some(FlagEntry { path, mtime, hash })
}

/// Save flag index for a given state (sorted by path)
pub fn save_index(root: &Path, state: &str, entries: &BTreeMap<String, FlagEntry>) -> Result<()> {
    let dir = cache_dir(root);
    fs::create_dir_all(&dir)?;
    let path = index_path(root, state);
    let mut file = fs::File::create(&path)?;
    // BTreeMap is already sorted by key (path)
    for entry in entries.values() {
        writeln!(file, "{} {} {}", entry.path, entry.mtime, entry.hash)?;
    }
    Ok(())
}

/// Compute sha256 hash prefix (first 10 hex chars) for a file
fn compute_hash(path: &Path) -> Result<String> {
    let content = fs::read(path)?;
    let digest = Sha256::digest(&content);
    let full_hex = hex::encode(digest);
    Ok(full_hex[..10].to_string())
}

/// Get file mtime as unix seconds
fn file_mtime(path: &Path) -> Result<u64> {
    let meta = fs::metadata(path)?;
    let mtime = meta
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::Other(e.to_string()))?
        .as_secs();
    Ok(mtime)
}

/// Flag files as clean for a given state.
/// Returns the entries that were flagged.
pub fn flag_files(
    root: &Path,
    config: &FlagsConfig,
    state: &str,
    paths: &[String],
) -> Result<Vec<FlagEntry>> {
    validate_state(config, state)?;
    let mut index = load_index(root, state)?;
    let mut flagged = Vec::new();

    for rel_path in paths {
        let abs_path = root.join(rel_path);
        if !abs_path.exists() {
            return Err(Error::Other(format!("file not found: {}", rel_path)));
        }
        if abs_path.is_dir() {
            return Err(Error::Other(format!("expected file, got directory: {}", rel_path)));
        }
        let mtime = file_mtime(&abs_path)?;
        let hash = compute_hash(&abs_path)?;
        let entry = FlagEntry {
            path: rel_path.clone(),
            mtime,
            hash,
        };
        index.insert(rel_path.clone(), entry.clone());
        flagged.push(entry);
    }

    save_index(root, state, &index)?;
    Ok(flagged)
}

/// Check status of all flagged files for a state
pub fn check_status(
    root: &Path,
    state: &str,
    check_mode: CheckMode,
) -> Result<Vec<(FlagEntry, FileStatus)>> {
    let index = load_index(root, state)?;
    let mut results = Vec::new();

    for entry in index.values() {
        let abs_path = root.join(&entry.path);
        if !abs_path.exists() {
            results.push((entry.clone(), FileStatus::Missing));
            continue;
        }

        let current_mtime = file_mtime(&abs_path)?;
        if current_mtime == entry.mtime {
            results.push((entry.clone(), FileStatus::Clean));
            continue;
        }

        // mtime differs
        match check_mode {
            CheckMode::Fast => {
                results.push((entry.clone(), FileStatus::Dirty));
            }
            CheckMode::Strict => {
                let current_hash = compute_hash(&abs_path)?;
                let status = if current_hash == entry.hash {
                    FileStatus::Clean
                } else {
                    FileStatus::Dirty
                };
                results.push((entry.clone(), status));
            }
        }
    }

    Ok(results)
}

/// Remove flag entries for specific files, or all if paths is empty
pub fn clear_flags(
    root: &Path,
    config: &FlagsConfig,
    state: &str,
    paths: &[String],
) -> Result<usize> {
    validate_state(config, state)?;
    let mut index = load_index(root, state)?;
    let removed = if paths.is_empty() {
        let count = index.len();
        index.clear();
        count
    } else {
        let mut count = 0;
        for p in paths {
            if index.remove(p).is_some() {
                count += 1;
            }
        }
        count
    };
    save_index(root, state, &index)?;
    Ok(removed)
}

/// Create default `.adi/flags.toml`
pub fn init_config(root: &Path) -> Result<()> {
    let path = config_path(root);
    if path.exists() {
        return Err(Error::Other(".adi/flags.toml already exists".to_string()));
    }
    let parent = path.parent().unwrap();
    fs::create_dir_all(parent)?;
    fs::write(
        &path,
        r#"# File flagger configuration
# check = "fast"   — mtime only (may false-trigger dirty)
# check = "strict" — mtime first, then hash to confirm (no false triggers)
check = "fast"

[states.reviewed]
description = "Code reviewed"

[states.tested]
description = "Manually tested"
"#,
    )?;
    Ok(())
}
