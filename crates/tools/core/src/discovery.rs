use crate::{parse_help_text, Config, Error, Result, Tool, ToolSource, ToolUsage};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Command;
use walkdir::WalkDir;

/// Discover all tools from configured sources
pub fn discover_all(config: &Config) -> Result<Vec<Tool>> {
    let mut tools = Vec::new();

    // 1. Scan ADI plugins
    #[cfg(feature = "plugin-discovery")]
    {
        match discover_plugins(&config.plugins_dir) {
            Ok(plugin_tools) => tools.extend(plugin_tools),
            Err(e) => tracing::warn!("Failed to discover plugins: {}", e),
        }
    }

    // 2. Scan tools directory
    match discover_tools_dir(&config.tools_dir) {
        Ok(dir_tools) => tools.extend(dir_tools),
        Err(e) => tracing::warn!("Failed to discover tools dir: {}", e),
    }

    Ok(tools)
}

/// Discover tools from ADI plugins directory
#[cfg(feature = "plugin-discovery")]
pub fn discover_plugins(plugins_dir: &Path) -> Result<Vec<Tool>> {
    use lib_plugin_manifest::PluginManifest;

    let mut tools = Vec::new();

    if !plugins_dir.exists() {
        return Ok(tools);
    }

    for entry in WalkDir::new(plugins_dir).max_depth(2) {
        let entry = entry.map_err(|e| Error::Discovery(e.to_string()))?;
        if entry.file_name() == "plugin.toml" {
            if let Ok(manifest) = PluginManifest::from_file(entry.path()) {
                if let Some(cli) = &manifest.cli {
                    let tool_id = format!("{}.{}", manifest.plugin.id, cli.command);
                    tools.push(Tool {
                        id: tool_id,
                        name: format!("adi {}", cli.command),
                        description: cli.description.clone(),
                        source: ToolSource::Plugin {
                            plugin_id: manifest.plugin.id.clone(),
                            command: cli.command.clone(),
                        },
                        updated_at: Utc::now().timestamp(),
                    });
                }
            }
        }
    }

    Ok(tools)
}

/// Stub for when plugin-discovery feature is disabled
#[cfg(not(feature = "plugin-discovery"))]
pub fn discover_plugins(_plugins_dir: &Path) -> Result<Vec<Tool>> {
    Ok(Vec::new())
}

/// Discover tools from ~/.local/share/adi/tools/
pub fn discover_tools_dir(tools_dir: &Path) -> Result<Vec<Tool>> {
    let mut tools = Vec::new();

    if !tools_dir.exists() {
        return Ok(tools);
    }

    for entry in std::fs::read_dir(tools_dir)? {
        let entry = entry?;
        let path = entry.path();

        if is_executable(&path) {
            match discover_tool_from_path(&path) {
                Ok(tool) => tools.push(tool),
                Err(e) => tracing::warn!("Failed to discover tool {:?}: {}", path, e),
            }
        }
    }

    Ok(tools)
}

/// Discover a single tool from its executable path
pub fn discover_tool_from_path(path: &Path) -> Result<Tool> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::Discovery("Invalid path".to_string()))?
        .to_string();

    let description = extract_description(path)?;
    let hash = hash_file(path)?;

    Ok(Tool {
        id: name.clone(),
        name,
        description,
        source: ToolSource::ToolDir {
            path: path.to_path_buf(),
            hash,
        },
        updated_at: Utc::now().timestamp(),
    })
}

/// Extract description from tool
fn extract_description(path: &Path) -> Result<String> {
    // Try: tool describe (single line output)
    if let Ok(output) = Command::new(path).arg("describe").output() {
        if output.status.success() {
            let desc = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !desc.is_empty() && desc.len() < 200 {
                return Ok(desc);
            }
        }
    }

    // Fallback: parse first paragraph from --help
    let output = Command::new(path)
        .arg("--help")
        .output()
        .map_err(|e| Error::Discovery(format!("Failed to run --help: {}", e)))?;

    let help = String::from_utf8_lossy(&output.stdout);
    Ok(parse_first_paragraph(&help))
}

/// Fetch full --help output for a tool
pub fn fetch_help(tool: &Tool) -> Result<ToolUsage> {
    let help_text = match &tool.source {
        ToolSource::Plugin { command, .. } => {
            // Run: adi <command> --help
            let output = Command::new("adi")
                .args([command, "--help"])
                .output()
                .map_err(|e| {
                    Error::Discovery(format!("Failed to run adi {} --help: {}", command, e))
                })?;
            String::from_utf8_lossy(&output.stdout).to_string()
        }
        ToolSource::ToolDir { path, .. } | ToolSource::System { path } => {
            let output = Command::new(path)
                .arg("--help")
                .output()
                .map_err(|e| Error::Discovery(format!("Failed to run --help: {}", e)))?;
            String::from_utf8_lossy(&output.stdout).to_string()
        }
    };

    let (examples, flags) = parse_help_text(&help_text);

    Ok(ToolUsage {
        tool_id: tool.id.clone(),
        help_text,
        examples,
        flags,
    })
}

fn parse_first_paragraph(help: &str) -> String {
    let lines: Vec<&str> = help.lines().collect();

    // Skip empty lines at start
    let mut start = 0;
    while start < lines.len() && lines[start].trim().is_empty() {
        start += 1;
    }

    // Find first paragraph (up to empty line or "Usage:" or "USAGE:")
    let mut end = start;
    while end < lines.len() {
        let line = lines[end].trim();
        if line.is_empty()
            || line.starts_with("Usage:")
            || line.starts_with("USAGE:")
            || line.starts_with("usage:")
        {
            break;
        }
        end += 1;
    }

    if start < end {
        lines[start..end].join(" ").trim().to_string()
    } else {
        "No description available".to_string()
    }
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = path.metadata() {
            return metadata.is_file() && (metadata.permissions().mode() & 0o111 != 0);
        }
    }
    #[cfg(windows)]
    {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            return ext == "exe" || ext == "bat" || ext == "cmd";
        }
    }
    false
}

fn hash_file(path: &Path) -> Result<String> {
    let content = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_first_paragraph() {
        let help = r#"
mytool - A simple tool for testing

Usage: mytool [OPTIONS]

Options:
  -h, --help  Show help
"#;
        let desc = parse_first_paragraph(help);
        assert_eq!(desc, "mytool - A simple tool for testing");
    }

    #[test]
    fn test_parse_first_paragraph_with_usage_first() {
        let help = r#"Usage: mytool [OPTIONS]

A simple tool for testing.
"#;
        let desc = parse_first_paragraph(help);
        // Should return empty or default since Usage is first
        assert_eq!(desc, "No description available");
    }
}
