use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A discovered CLI tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: ToolSource,
    pub updated_at: i64,
}

/// Where the tool came from
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolSource {
    /// ADI plugin command
    Plugin { plugin_id: String, command: String },
    /// Executable in ~/.local/share/adi/tools/
    ToolDir { path: PathBuf, hash: String },
    /// System executable (git, docker, etc.)
    System { path: PathBuf },
}

/// Full usage information for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsage {
    pub tool_id: String,
    pub help_text: String,
    pub examples: Vec<String>,
    pub flags: Vec<ToolFlag>,
}

/// A parsed flag from --help
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFlag {
    pub short: Option<String>,
    pub long: Option<String>,
    pub description: String,
    pub takes_value: bool,
}

/// Search result with relevance score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub tool: Tool,
    pub score: f32,
    pub match_type: MatchType,
}

/// How the tool matched the query
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    Exact,
    Fuzzy,
    Semantic,
    Keyword,
}

/// Tool index configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: PathBuf,
    pub tools_dir: PathBuf,
    pub plugins_dir: PathBuf,
    pub scan_system: bool,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("adi");

        Self {
            db_path: data_dir.join("tools.db"),
            tools_dir: data_dir.join("tools"),
            plugins_dir: data_dir.join("plugins"),
            scan_system: false,
        }
    }
}
