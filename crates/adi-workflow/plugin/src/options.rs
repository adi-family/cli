//! Dynamic options providers for workflow inputs
//!
//! This module provides various ways to generate options dynamically:
//! - Shell commands (options_cmd)
//! - Built-in providers (options_source)
//! - Static options (options)

use crate::parser::{Input, OptionsSource};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Resolve options for an input, checking dynamic sources first
pub fn resolve_options(
    input: &Input,
    values: &HashMap<String, serde_json::Value>,
) -> Result<Vec<String>, String> {
    // Priority: options_cmd > options_source > options (static)

    // 1. Try shell command
    if let Some(cmd) = &input.options_cmd {
        return resolve_from_command(cmd, values);
    }

    // 2. Try built-in source
    if let Some(source) = &input.options_source {
        return resolve_from_source(source);
    }

    // 3. Fall back to static options
    if let Some(opts) = &input.options {
        return Ok(opts.clone());
    }

    Err(format!(
        "Input '{}' requires options, options_cmd, or options_source",
        input.name
    ))
}

/// Resolve options from a shell command
fn resolve_from_command(
    cmd: &str,
    _values: &HashMap<String, serde_json::Value>,
) -> Result<Vec<String>, String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| format!("Failed to execute options_cmd: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("options_cmd failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let options: Vec<String> = stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if options.is_empty() {
        return Err("options_cmd returned no options".to_string());
    }

    Ok(options)
}

/// Resolve options from a built-in source
fn resolve_from_source(source: &OptionsSource) -> Result<Vec<String>, String> {
    match source {
        OptionsSource::GitBranches => get_git_branches(),
        OptionsSource::GitTags => get_git_tags(),
        OptionsSource::GitRemotes => get_git_remotes(),
        OptionsSource::Plugins => get_plugins(),
        OptionsSource::DockerComposeServices { file } => get_docker_compose_services(file),
        OptionsSource::Directories { path, pattern } => get_directories(path, pattern.as_deref()),
        OptionsSource::Files { path, pattern } => get_files(path, pattern.as_deref()),
        OptionsSource::LinesFromFile { path } => get_lines_from_file(path),
        OptionsSource::CargoWorkspaceMembers => get_cargo_workspace_members(),
        OptionsSource::ReleaseServices => get_release_services(),
    }
}

/// Get git branches from current repository
fn get_git_branches() -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .output()
        .map_err(|e| format!("Failed to get git branches: {}", e))?;

    if !output.status.success() {
        return Err("Not a git repository or git not installed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let branches: Vec<String> = stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if branches.is_empty() {
        return Err("No git branches found".to_string());
    }

    Ok(branches)
}

/// Get git tags from current repository
fn get_git_tags() -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["tag", "--sort=-version:refname"])
        .output()
        .map_err(|e| format!("Failed to get git tags: {}", e))?;

    if !output.status.success() {
        return Err("Not a git repository or git not installed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let tags: Vec<String> = stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if tags.is_empty() {
        return Err("No git tags found".to_string());
    }

    Ok(tags)
}

/// Get git remotes from current repository
fn get_git_remotes() -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["remote"])
        .output()
        .map_err(|e| format!("Failed to get git remotes: {}", e))?;

    if !output.status.success() {
        return Err("Not a git repository or git not installed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let remotes: Vec<String> = stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if remotes.is_empty() {
        return Err("No git remotes found".to_string());
    }

    Ok(remotes)
}

/// Get plugin directories (crates containing plugin.toml)
fn get_plugins() -> Result<Vec<String>, String> {
    let mut plugins = Vec::new();

    // Look for plugin.toml files in crates/
    // Supports: crates/foo/plugin.toml and crates/foo/plugin/plugin.toml
    if let Ok(entries) = std::fs::read_dir("crates") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Check for plugin.toml directly in the crate
                let plugin_toml = path.join("plugin.toml");
                // Check for plugin.toml in a plugin/ subdirectory
                let plugin_subdir_toml = path.join("plugin").join("plugin.toml");

                if plugin_toml.exists() || plugin_subdir_toml.exists() {
                    if let Some(name) = path.file_name() {
                        plugins.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    plugins.sort();

    if plugins.is_empty() {
        return Err("No plugins found in crates/".to_string());
    }

    Ok(plugins)
}

/// Get services from docker-compose.yml
fn get_docker_compose_services(file: &str) -> Result<Vec<String>, String> {
    // Try to parse with docker-compose config
    let output = Command::new("docker")
        .args(["compose", "-f", file, "config", "--services"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let services: Vec<String> = stdout
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if !services.is_empty() {
                return Ok(services);
            }
        }
    }

    // Fallback: parse YAML manually (basic)
    let content =
        std::fs::read_to_string(file).map_err(|e| format!("Failed to read {}: {}", file, e))?;

    let mut services = Vec::new();
    let mut in_services = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "services:" {
            in_services = true;
            continue;
        }

        if in_services {
            // Check if this is a top-level key under services (2 spaces indent)
            if line.starts_with("  ") && !line.starts_with("    ") && trimmed.ends_with(':') {
                let service = trimmed.trim_end_matches(':').to_string();
                if !service.starts_with('#') {
                    services.push(service);
                }
            }

            // Check if we've left the services section
            if !line.starts_with(' ') && !trimmed.is_empty() && trimmed != "services:" {
                break;
            }
        }
    }

    if services.is_empty() {
        return Err(format!("No services found in {}", file));
    }

    Ok(services)
}

/// Get directories matching a pattern
fn get_directories(base_path: &str, pattern: Option<&str>) -> Result<Vec<String>, String> {
    let path = Path::new(base_path);

    if !path.exists() {
        return Err(format!("Directory not found: {}", base_path));
    }

    let mut dirs = Vec::new();

    if let Some(pat) = pattern {
        // Use glob pattern
        let glob_pattern = format!("{}/{}", base_path, pat);
        for entry in
            glob::glob(&glob_pattern).map_err(|e| format!("Invalid glob pattern: {}", e))?
        {
            if let Ok(path) = entry {
                if path.is_dir() {
                    if let Some(name) = path.file_name() {
                        dirs.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    } else {
        // List all directories
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.path().file_name() {
                        dirs.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    dirs.sort();

    if dirs.is_empty() {
        return Err(format!(
            "No directories found in {} matching {:?}",
            base_path, pattern
        ));
    }

    Ok(dirs)
}

/// Get files matching a pattern
fn get_files(base_path: &str, pattern: Option<&str>) -> Result<Vec<String>, String> {
    let path = Path::new(base_path);

    if !path.exists() {
        return Err(format!("Path not found: {}", base_path));
    }

    let mut files = Vec::new();

    if let Some(pat) = pattern {
        // Use glob pattern
        let glob_pattern = format!("{}/{}", base_path, pat);
        for entry in
            glob::glob(&glob_pattern).map_err(|e| format!("Invalid glob pattern: {}", e))?
        {
            if let Ok(path) = entry {
                if path.is_file() {
                    if let Some(name) = path.file_name() {
                        files.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    } else {
        // List all files
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if entry.path().is_file() {
                    if let Some(name) = entry.path().file_name() {
                        files.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    files.sort();

    if files.is_empty() {
        return Err(format!(
            "No files found in {} matching {:?}",
            base_path, pattern
        ));
    }

    Ok(files)
}

/// Get lines from a file (one option per line)
fn get_lines_from_file(path: &str) -> Result<Vec<String>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;

    let lines: Vec<String> = content
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .collect();

    if lines.is_empty() {
        return Err(format!("No options found in {}", path));
    }

    Ok(lines)
}

/// Get cargo workspace members
fn get_cargo_workspace_members() -> Result<Vec<String>, String> {
    // Try cargo metadata first
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version=1"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(packages) = metadata["packages"].as_array() {
                    let members: Vec<String> = packages
                        .iter()
                        .filter_map(|p| p["name"].as_str().map(String::from))
                        .collect();

                    if !members.is_empty() {
                        return Ok(members);
                    }
                }
            }
        }
    }

    // Fallback: parse Cargo.toml manually
    let content = std::fs::read_to_string("Cargo.toml")
        .map_err(|_| "No Cargo.toml found in current directory".to_string())?;

    let toml: toml::Value =
        toml::from_str(&content).map_err(|e| format!("Failed to parse Cargo.toml: {}", e))?;

    let mut members = Vec::new();

    if let Some(workspace) = toml.get("workspace") {
        if let Some(member_list) = workspace.get("members").and_then(|m| m.as_array()) {
            for member in member_list {
                if let Some(m) = member.as_str() {
                    // Handle glob patterns
                    if m.contains('*') {
                        if let Ok(entries) = glob::glob(m) {
                            for entry in entries.flatten() {
                                if let Some(name) = entry.file_name() {
                                    members.push(name.to_string_lossy().to_string());
                                }
                            }
                        }
                    } else {
                        let path = Path::new(m);
                        if let Some(name) = path.file_name() {
                            members.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    if members.is_empty() {
        return Err("No workspace members found".to_string());
    }

    members.sort();
    Ok(members)
}

/// Get release services from release/ directory
fn get_release_services() -> Result<Vec<String>, String> {
    let release_dir = Path::new("release");

    if !release_dir.exists() {
        return Err("No release/ directory found".to_string());
    }

    let mut services = Vec::new();

    // Look for subdirectories that contain Dockerfile
    for entry in
        std::fs::read_dir(release_dir).map_err(|e| format!("Failed to read release/: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            // Check if this directory or any subdirectory has a Dockerfile
            let has_dockerfile = path.join("Dockerfile").exists()
                || std::fs::read_dir(&path)
                    .ok()
                    .map(|entries| {
                        entries
                            .flatten()
                            .any(|e| e.path().is_dir() && e.path().join("Dockerfile").exists())
                    })
                    .unwrap_or(false);

            if has_dockerfile {
                if let Some(name) = path.file_name() {
                    services.push(name.to_string_lossy().to_string());
                }
            }

            // Also check for nested services (e.g., release/domain.com/service/)
            if let Ok(subentries) = std::fs::read_dir(&path) {
                for subentry in subentries.flatten() {
                    if subentry.path().is_dir() && subentry.path().join("Dockerfile").exists() {
                        if let Some(name) = subentry.path().file_name() {
                            services.push(name.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    services.sort();
    services.dedup();

    if services.is_empty() {
        return Err("No services found in release/ directory".to_string());
    }

    Ok(services)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_static_options() {
        let input = Input {
            name: "test".to_string(),
            input_type: crate::parser::InputType::Select,
            prompt: "Test".to_string(),
            options: Some(vec!["a".to_string(), "b".to_string()]),
            options_cmd: None,
            options_source: None,
            autocomplete: None,
            default: None,
            validation: None,
            env: None,
            condition: None,
        };

        let values = HashMap::new();
        let result = resolve_options(&input, &values).unwrap();
        assert_eq!(result, vec!["a", "b"]);
    }

    #[test]
    fn test_resolve_from_command() {
        let result = resolve_from_command("printf 'a\nb\nc\n'", &HashMap::new());
        assert!(result.is_ok());
        let options = result.unwrap();
        assert_eq!(options, vec!["a", "b", "c"]);
    }
}
