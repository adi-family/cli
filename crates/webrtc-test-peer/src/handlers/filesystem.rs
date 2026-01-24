//! FileSystem mock handler
//!
//! Simulates filesystem operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::MessageHandler;
use crate::config::FilesystemScenario;

/// FileSystem request message (from browser)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileSystemRequest {
    FsListDir {
        request_id: String,
        path: String,
    },
    FsReadFile {
        request_id: String,
        path: String,
        #[serde(default)]
        offset: Option<u64>,
        #[serde(default)]
        limit: Option<u64>,
    },
    FsStat {
        request_id: String,
        path: String,
    },
    FsWalk {
        request_id: String,
        path: String,
        #[serde(default)]
        max_depth: Option<u32>,
        #[serde(default)]
        pattern: Option<String>,
    },
}

/// FileSystem response message (to browser)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileSystemResponse {
    FsDirListing {
        request_id: String,
        path: String,
        entries: Vec<FileEntry>,
    },
    FsFileContent {
        request_id: String,
        path: String,
        content: String,
        encoding: String,
        total_size: u64,
    },
    FsFileStat {
        request_id: String,
        path: String,
        stat: FileStat,
    },
    FsWalkResult {
        request_id: String,
        path: String,
        entries: Vec<WalkEntry>,
        truncated: bool,
    },
    FsError {
        request_id: String,
        code: String,
        message: String,
    },
}

/// File entry for directory listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
}

/// File stat information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStat {
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<u32>,
}

/// Walk entry with depth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalkEntry {
    pub path: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub depth: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

/// Virtual file/directory
#[derive(Debug, Clone)]
struct VirtualEntry {
    is_dir: bool,
    content: Option<String>,
    size: u64,
    children: Vec<String>,
}

/// FileSystem handler for simulating filesystem operations
pub struct FileSystemHandler {
    scenario: FilesystemScenario,
    fs_root: Option<PathBuf>,
    virtual_fs: Arc<Mutex<HashMap<String, VirtualEntry>>>,
}

impl FileSystemHandler {
    pub fn new(scenario: FilesystemScenario, fs_root: Option<PathBuf>) -> Self {
        let mut virtual_fs = HashMap::new();

        // Build virtual filesystem from scenario
        for entry in &scenario.virtual_fs {
            virtual_fs.insert(
                entry.path.clone(),
                VirtualEntry {
                    is_dir: entry.is_dir,
                    content: entry.content.clone(),
                    size: entry.size.unwrap_or_else(|| {
                        entry.content.as_ref().map(|c| c.len() as u64).unwrap_or(0)
                    }),
                    children: entry.children.clone(),
                },
            );
        }

        // Add default structure if empty
        if virtual_fs.is_empty() {
            Self::add_default_fs(&mut virtual_fs);
        }

        Self {
            scenario,
            fs_root,
            virtual_fs: Arc::new(Mutex::new(virtual_fs)),
        }
    }

    fn add_default_fs(fs: &mut HashMap<String, VirtualEntry>) {
        // Root
        fs.insert(
            "/".to_string(),
            VirtualEntry {
                is_dir: true,
                content: None,
                size: 4096,
                children: vec!["home".to_string(), "tmp".to_string(), "etc".to_string()],
            },
        );

        // /home
        fs.insert(
            "/home".to_string(),
            VirtualEntry {
                is_dir: true,
                content: None,
                size: 4096,
                children: vec!["testuser".to_string()],
            },
        );

        // /home/testuser
        fs.insert(
            "/home/testuser".to_string(),
            VirtualEntry {
                is_dir: true,
                content: None,
                size: 4096,
                children: vec![
                    "file1.txt".to_string(),
                    "file2.txt".to_string(),
                    "dir1".to_string(),
                ],
            },
        );

        // Files in /home/testuser
        fs.insert(
            "/home/testuser/file1.txt".to_string(),
            VirtualEntry {
                is_dir: false,
                content: Some("Hello, World!\nThis is file1.\n".to_string()),
                size: 30,
                children: vec![],
            },
        );

        fs.insert(
            "/home/testuser/file2.txt".to_string(),
            VirtualEntry {
                is_dir: false,
                content: Some("File 2 content.\nLine 2.\nLine 3.\n".to_string()),
                size: 35,
                children: vec![],
            },
        );

        fs.insert(
            "/home/testuser/dir1".to_string(),
            VirtualEntry {
                is_dir: true,
                content: None,
                size: 4096,
                children: vec!["nested.txt".to_string()],
            },
        );

        fs.insert(
            "/home/testuser/dir1/nested.txt".to_string(),
            VirtualEntry {
                is_dir: false,
                content: Some("Nested file content.\n".to_string()),
                size: 21,
                children: vec![],
            },
        );

        // /tmp
        fs.insert(
            "/tmp".to_string(),
            VirtualEntry {
                is_dir: true,
                content: None,
                size: 4096,
                children: vec![],
            },
        );

        // /etc
        fs.insert(
            "/etc".to_string(),
            VirtualEntry {
                is_dir: true,
                content: None,
                size: 4096,
                children: vec!["hostname".to_string()],
            },
        );

        fs.insert(
            "/etc/hostname".to_string(),
            VirtualEntry {
                is_dir: false,
                content: Some("test-cocoon\n".to_string()),
                size: 12,
                children: vec![],
            },
        );
    }

    fn handle_request(&self, req: FileSystemRequest) -> FileSystemResponse {
        match req {
            FileSystemRequest::FsListDir { request_id, path } => {
                // Check if path should fail
                if self.scenario.fail_paths.iter().any(|p| path.starts_with(p)) {
                    return FileSystemResponse::FsError {
                        request_id,
                        code: "EACCES".to_string(),
                        message: format!("Permission denied: {}", path),
                    };
                }

                // Try real filesystem first if configured
                if let Some(root) = &self.fs_root {
                    let full_path = root.join(path.trim_start_matches('/'));
                    if full_path.exists() && full_path.is_dir() {
                        return self.list_real_dir(&request_id, &path, &full_path);
                    }
                }

                // Fall back to virtual filesystem
                self.list_virtual_dir(&request_id, &path)
            }

            FileSystemRequest::FsReadFile {
                request_id,
                path,
                offset,
                limit,
            } => {
                if self.scenario.fail_paths.iter().any(|p| path.starts_with(p)) {
                    return FileSystemResponse::FsError {
                        request_id,
                        code: "EACCES".to_string(),
                        message: format!("Permission denied: {}", path),
                    };
                }

                // Try real filesystem first
                if let Some(root) = &self.fs_root {
                    let full_path = root.join(path.trim_start_matches('/'));
                    if full_path.exists() && full_path.is_file() {
                        return self.read_real_file(&request_id, &path, &full_path, offset, limit);
                    }
                }

                self.read_virtual_file(&request_id, &path, offset, limit)
            }

            FileSystemRequest::FsStat { request_id, path } => {
                if self.scenario.fail_paths.iter().any(|p| path.starts_with(p)) {
                    return FileSystemResponse::FsError {
                        request_id,
                        code: "EACCES".to_string(),
                        message: format!("Permission denied: {}", path),
                    };
                }

                // Try real filesystem first
                if let Some(root) = &self.fs_root {
                    let full_path = root.join(path.trim_start_matches('/'));
                    if full_path.exists() {
                        return self.stat_real_path(&request_id, &path, &full_path);
                    }
                }

                self.stat_virtual_path(&request_id, &path)
            }

            FileSystemRequest::FsWalk {
                request_id,
                path,
                max_depth,
                pattern: _,
            } => {
                if self.scenario.fail_paths.iter().any(|p| path.starts_with(p)) {
                    return FileSystemResponse::FsError {
                        request_id,
                        code: "EACCES".to_string(),
                        message: format!("Permission denied: {}", path),
                    };
                }

                self.walk_virtual_path(&request_id, &path, max_depth.unwrap_or(10))
            }
        }
    }

    fn list_virtual_dir(&self, request_id: &str, path: &str) -> FileSystemResponse {
        let fs = self.virtual_fs.lock().unwrap();

        let normalized = if path.is_empty() || path == "." {
            "/"
        } else {
            path
        };

        if let Some(entry) = fs.get(normalized) {
            if !entry.is_dir {
                return FileSystemResponse::FsError {
                    request_id: request_id.to_string(),
                    code: "ENOTDIR".to_string(),
                    message: format!("Not a directory: {}", path),
                };
            }

            let entries: Vec<FileEntry> = entry
                .children
                .iter()
                .map(|name| {
                    let child_path = if normalized == "/" {
                        format!("/{}", name)
                    } else {
                        format!("{}/{}", normalized, name)
                    };

                    let child = fs.get(&child_path);
                    FileEntry {
                        name: name.clone(),
                        is_dir: child.map(|c| c.is_dir).unwrap_or(false),
                        is_file: child.map(|c| !c.is_dir).unwrap_or(false),
                        is_symlink: false,
                        size: child.map(|c| c.size),
                        modified: Some("2026-01-24T12:00:00Z".to_string()),
                    }
                })
                .collect();

            FileSystemResponse::FsDirListing {
                request_id: request_id.to_string(),
                path: path.to_string(),
                entries,
            }
        } else {
            FileSystemResponse::FsError {
                request_id: request_id.to_string(),
                code: "ENOENT".to_string(),
                message: format!("No such file or directory: {}", path),
            }
        }
    }

    fn read_virtual_file(
        &self,
        request_id: &str,
        path: &str,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> FileSystemResponse {
        let fs = self.virtual_fs.lock().unwrap();

        if let Some(entry) = fs.get(path) {
            if entry.is_dir {
                return FileSystemResponse::FsError {
                    request_id: request_id.to_string(),
                    code: "EISDIR".to_string(),
                    message: format!("Is a directory: {}", path),
                };
            }

            let content = entry.content.clone().unwrap_or_default();
            let total_size = content.len() as u64;

            let offset = offset.unwrap_or(0) as usize;
            let limit = limit.unwrap_or(total_size) as usize;

            let sliced = if offset < content.len() {
                let end = std::cmp::min(offset + limit, content.len());
                &content[offset..end]
            } else {
                ""
            };

            FileSystemResponse::FsFileContent {
                request_id: request_id.to_string(),
                path: path.to_string(),
                content: sliced.to_string(),
                encoding: "utf8".to_string(),
                total_size,
            }
        } else {
            FileSystemResponse::FsError {
                request_id: request_id.to_string(),
                code: "ENOENT".to_string(),
                message: format!("No such file or directory: {}", path),
            }
        }
    }

    fn stat_virtual_path(&self, request_id: &str, path: &str) -> FileSystemResponse {
        let fs = self.virtual_fs.lock().unwrap();

        let normalized = if path.is_empty() || path == "." {
            "/"
        } else {
            path
        };

        if let Some(entry) = fs.get(normalized) {
            FileSystemResponse::FsFileStat {
                request_id: request_id.to_string(),
                path: path.to_string(),
                stat: FileStat {
                    is_dir: entry.is_dir,
                    is_file: !entry.is_dir,
                    is_symlink: false,
                    size: entry.size,
                    modified: Some("2026-01-24T12:00:00Z".to_string()),
                    created: Some("2026-01-24T12:00:00Z".to_string()),
                    permissions: Some(if entry.is_dir { 0o755 } else { 0o644 }),
                },
            }
        } else {
            FileSystemResponse::FsError {
                request_id: request_id.to_string(),
                code: "ENOENT".to_string(),
                message: format!("No such file or directory: {}", path),
            }
        }
    }

    fn walk_virtual_path(
        &self,
        request_id: &str,
        path: &str,
        max_depth: u32,
    ) -> FileSystemResponse {
        let fs = self.virtual_fs.lock().unwrap();
        let mut entries = Vec::new();
        let mut to_visit = vec![(path.to_string(), 0u32)];

        while let Some((current_path, depth)) = to_visit.pop() {
            if depth > max_depth {
                continue;
            }

            if let Some(entry) = fs.get(&current_path) {
                entries.push(WalkEntry {
                    path: current_path.clone(),
                    is_dir: entry.is_dir,
                    is_file: !entry.is_dir,
                    depth,
                    size: if entry.is_dir { None } else { Some(entry.size) },
                });

                if entry.is_dir {
                    for child in &entry.children {
                        let child_path = if current_path == "/" {
                            format!("/{}", child)
                        } else {
                            format!("{}/{}", current_path, child)
                        };
                        to_visit.push((child_path, depth + 1));
                    }
                }
            }
        }

        FileSystemResponse::FsWalkResult {
            request_id: request_id.to_string(),
            path: path.to_string(),
            entries,
            truncated: false,
        }
    }

    fn list_real_dir(
        &self,
        request_id: &str,
        path: &str,
        full_path: &PathBuf,
    ) -> FileSystemResponse {
        match std::fs::read_dir(full_path) {
            Ok(entries) => {
                let entries: Vec<FileEntry> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| {
                        let metadata = e.metadata().ok();
                        FileEntry {
                            name: e.file_name().to_string_lossy().to_string(),
                            is_dir: metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                            is_file: metadata.as_ref().map(|m| m.is_file()).unwrap_or(false),
                            is_symlink: metadata
                                .as_ref()
                                .map(|m| m.file_type().is_symlink())
                                .unwrap_or(false),
                            size: metadata.as_ref().map(|m| m.len()),
                            modified: None, // Could add real mtime
                        }
                    })
                    .collect();

                FileSystemResponse::FsDirListing {
                    request_id: request_id.to_string(),
                    path: path.to_string(),
                    entries,
                }
            }
            Err(e) => FileSystemResponse::FsError {
                request_id: request_id.to_string(),
                code: "EIO".to_string(),
                message: format!("Failed to read directory: {}", e),
            },
        }
    }

    fn read_real_file(
        &self,
        request_id: &str,
        path: &str,
        full_path: &PathBuf,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> FileSystemResponse {
        match std::fs::read(full_path) {
            Ok(bytes) => {
                let total_size = bytes.len() as u64;
                let offset = offset.unwrap_or(0) as usize;
                let limit = limit.unwrap_or(total_size) as usize;

                let sliced = if offset < bytes.len() {
                    let end = std::cmp::min(offset + limit, bytes.len());
                    &bytes[offset..end]
                } else {
                    &[]
                };

                // Try to decode as UTF-8, fall back to base64
                let (content, encoding) = match std::str::from_utf8(sliced) {
                    Ok(s) => (s.to_string(), "utf8"),
                    Err(_) => (
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, sliced),
                        "base64",
                    ),
                };

                FileSystemResponse::FsFileContent {
                    request_id: request_id.to_string(),
                    path: path.to_string(),
                    content,
                    encoding: encoding.to_string(),
                    total_size,
                }
            }
            Err(e) => FileSystemResponse::FsError {
                request_id: request_id.to_string(),
                code: "EIO".to_string(),
                message: format!("Failed to read file: {}", e),
            },
        }
    }

    fn stat_real_path(
        &self,
        request_id: &str,
        path: &str,
        full_path: &PathBuf,
    ) -> FileSystemResponse {
        match std::fs::metadata(full_path) {
            Ok(m) => FileSystemResponse::FsFileStat {
                request_id: request_id.to_string(),
                path: path.to_string(),
                stat: FileStat {
                    is_dir: m.is_dir(),
                    is_file: m.is_file(),
                    is_symlink: m.file_type().is_symlink(),
                    size: m.len(),
                    modified: None,
                    created: None,
                    permissions: None,
                },
            },
            Err(e) => FileSystemResponse::FsError {
                request_id: request_id.to_string(),
                code: "EIO".to_string(),
                message: format!("Failed to stat: {}", e),
            },
        }
    }
}

impl MessageHandler for FileSystemHandler {
    fn handle(&self, data: &str) -> Option<String> {
        let req: FileSystemRequest = serde_json::from_str(data).ok()?;
        let response = self.handle_request(req);
        serde_json::to_string(&response).ok()
    }

    fn channel(&self) -> &'static str {
        "file"
    }
}
