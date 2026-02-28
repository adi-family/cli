//! Dynamic options providers for workflow inputs

use crate::parser::{Input, OptionsSource};
use lib_plugin_prelude::t;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

pub fn resolve_options(
    input: &Input,
    values: &HashMap<String, serde_json::Value>,
) -> Result<Vec<String>, String> {
    if let Some(cmd) = &input.options_cmd {
        return resolve_from_command(cmd, values);
    }

    if let Some(source) = &input.options_source {
        return resolve_from_source(source);
    }

    if let Some(opts) = &input.options {
        return Ok(opts.clone());
    }

    Err(t!(
        "workflow-options-error-requires",
        "name" => input.name.as_str()
    ))
}

fn resolve_from_command(
    cmd: &str,
    _values: &HashMap<String, serde_json::Value>,
) -> Result<Vec<String>, String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| t!("workflow-options-error-cmd-exec", "error" => e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(t!(
            "workflow-options-error-cmd-failed",
            "error" => stderr.trim().to_string()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let options: Vec<String> = stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if options.is_empty() {
        return Err(t!("workflow-options-error-cmd-empty"));
    }

    Ok(options)
}

fn resolve_from_source(source: &OptionsSource) -> Result<Vec<String>, String> {
    match source {
        OptionsSource::GitBranches => get_git_branches(),
        OptionsSource::GitTags => get_git_tags(),
        OptionsSource::GitRemotes => get_git_remotes(),
        OptionsSource::DockerComposeServices { file } => get_docker_compose_services(file),
        OptionsSource::Directories { path, pattern } => get_directories(path, pattern.as_deref()),
        OptionsSource::Files { path, pattern } => get_files(path, pattern.as_deref()),
        OptionsSource::LinesFromFile { path } => get_lines_from_file(path),
        OptionsSource::CargoWorkspaceMembers => get_cargo_workspace_members(),
    }
}

fn get_git_branches() -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .output()
        .map_err(|e| t!("workflow-options-error-git-branches", "error" => e.to_string()))?;

    if !output.status.success() {
        return Err(t!("workflow-options-error-not-git"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let branches: Vec<String> = stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if branches.is_empty() {
        return Err(t!("workflow-options-error-no-branches"));
    }

    Ok(branches)
}

fn get_git_tags() -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["tag", "--sort=-version:refname"])
        .output()
        .map_err(|e| t!("workflow-options-error-git-tags", "error" => e.to_string()))?;

    if !output.status.success() {
        return Err(t!("workflow-options-error-not-git"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let tags: Vec<String> = stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if tags.is_empty() {
        return Err(t!("workflow-options-error-no-tags"));
    }

    Ok(tags)
}

fn get_git_remotes() -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["remote"])
        .output()
        .map_err(|e| t!("workflow-options-error-git-remotes", "error" => e.to_string()))?;

    if !output.status.success() {
        return Err(t!("workflow-options-error-not-git"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let remotes: Vec<String> = stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if remotes.is_empty() {
        return Err(t!("workflow-options-error-no-remotes"));
    }

    Ok(remotes)
}

fn get_docker_compose_services(file: &str) -> Result<Vec<String>, String> {
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

    // Fallback: basic YAML parsing
    let content = std::fs::read_to_string(file)
        .map_err(|e| t!("workflow-options-error-read-file", "path" => file, "error" => e.to_string()))?;

    let mut services = Vec::new();
    let mut in_services = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "services:" {
            in_services = true;
            continue;
        }

        if in_services {
            if line.starts_with("  ") && !line.starts_with("    ") && trimmed.ends_with(':') {
                let service = trimmed.trim_end_matches(':').to_string();
                if !service.starts_with('#') {
                    services.push(service);
                }
            }

            if !line.starts_with(' ') && !trimmed.is_empty() && trimmed != "services:" {
                break;
            }
        }
    }

    if services.is_empty() {
        return Err(t!("workflow-options-error-no-services", "file" => file));
    }

    Ok(services)
}

fn get_directories(base_path: &str, pattern: Option<&str>) -> Result<Vec<String>, String> {
    let path = Path::new(base_path);

    if !path.exists() {
        return Err(t!("workflow-options-error-dir-not-found", "path" => base_path));
    }

    let mut dirs = Vec::new();

    if let Some(pat) = pattern {
        let glob_pattern = format!("{}/{}", base_path, pat);
        for entry in
            glob::glob(&glob_pattern).map_err(|e| t!("workflow-options-error-glob", "error" => e.to_string()))?
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
        return Err(t!("workflow-options-error-no-dirs", "path" => base_path));
    }

    Ok(dirs)
}

fn get_files(base_path: &str, pattern: Option<&str>) -> Result<Vec<String>, String> {
    let path = Path::new(base_path);

    if !path.exists() {
        return Err(t!("workflow-options-error-path-not-found", "path" => base_path));
    }

    let mut files = Vec::new();

    if let Some(pat) = pattern {
        let glob_pattern = format!("{}/{}", base_path, pat);
        for entry in
            glob::glob(&glob_pattern).map_err(|e| t!("workflow-options-error-glob", "error" => e.to_string()))?
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
        return Err(t!("workflow-options-error-no-files", "path" => base_path));
    }

    Ok(files)
}

fn get_lines_from_file(path: &str) -> Result<Vec<String>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| t!("workflow-options-error-read-file", "path" => path, "error" => e.to_string()))?;

    let lines: Vec<String> = content
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .collect();

    if lines.is_empty() {
        return Err(t!("workflow-options-error-no-lines", "path" => path));
    }

    Ok(lines)
}

fn get_cargo_workspace_members() -> Result<Vec<String>, String> {
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

    // Fallback: parse Cargo.toml directly
    let content = std::fs::read_to_string("Cargo.toml")
        .map_err(|_| t!("workflow-options-error-no-cargo"))?;

    let toml: toml::Value =
        toml::from_str(&content).map_err(|e| t!("workflow-options-error-cargo-parse", "error" => e.to_string()))?;

    let mut members = Vec::new();

    if let Some(workspace) = toml.get("workspace") {
        if let Some(member_list) = workspace.get("members").and_then(|m| m.as_array()) {
            for member in member_list {
                if let Some(m) = member.as_str() {
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
        return Err(t!("workflow-options-error-no-members"));
    }

    members.sort();
    Ok(members)
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
            autocomplete_count: None,
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
