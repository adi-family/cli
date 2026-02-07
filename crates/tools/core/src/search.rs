use crate::{Config, MatchType, Result, SearchResult, Storage, Tool};
use std::cmp::Ordering;
use std::path::Path;

pub struct ToolSearch {
    storage: Storage,
}

impl ToolSearch {
    pub fn open(config: &Config) -> Result<Self> {
        if let Some(parent) = config.db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let storage = Storage::open(&config.db_path)?;

        Ok(Self { storage })
    }

    pub fn open_path(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let storage = Storage::open(path)?;
        Ok(Self { storage })
    }

    pub fn open_in_memory() -> Result<Self> {
        let storage = Storage::open_in_memory()?;
        Ok(Self { storage })
    }

    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    pub fn storage_mut(&mut self) -> &mut Storage {
        &mut self.storage
    }

    /// Find tools matching query
    pub fn find(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();

        // 1. Exact name match (highest priority)
        let all_tools = self.storage.list_tools()?;
        for tool in &all_tools {
            if tool.name.to_lowercase() == query_lower || tool.id.to_lowercase() == query_lower {
                results.push(SearchResult {
                    tool: tool.clone(),
                    score: 1.0,
                    match_type: MatchType::Exact,
                });
            }
        }

        // 2. Fuzzy name match (prefix, contains)
        for tool in &all_tools {
            let name_lower = tool.name.to_lowercase();
            let id_lower = tool.id.to_lowercase();

            // Skip if already matched exactly
            if results.iter().any(|r| r.tool.id == tool.id) {
                continue;
            }

            // Prefix match
            if name_lower.starts_with(&query_lower) || id_lower.starts_with(&query_lower) {
                results.push(SearchResult {
                    tool: tool.clone(),
                    score: 0.9,
                    match_type: MatchType::Fuzzy,
                });
                continue;
            }

            // Contains match
            if name_lower.contains(&query_lower) || id_lower.contains(&query_lower) {
                results.push(SearchResult {
                    tool: tool.clone(),
                    score: 0.7,
                    match_type: MatchType::Fuzzy,
                });
                continue;
            }

            // Word match in description
            let desc_lower = tool.description.to_lowercase();
            let word_matches = query_words
                .iter()
                .filter(|w| desc_lower.contains(*w))
                .count();

            if word_matches > 0 {
                let score = 0.5 * (word_matches as f32 / query_words.len() as f32);
                results.push(SearchResult {
                    tool: tool.clone(),
                    score,
                    match_type: MatchType::Keyword,
                });
            }
        }

        // 3. FTS search on description (if query has multiple words)
        if query_words.len() > 1 || results.len() < limit {
            if let Ok(fts_results) = self.storage.search_fts(query, limit) {
                for result in fts_results {
                    if !results.iter().any(|r| r.tool.id == result.tool.id) {
                        results.push(result);
                    }
                }
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    /// List all indexed tools
    pub fn list(&self) -> Result<Vec<Tool>> {
        self.storage.list_tools()
    }

    /// Get a specific tool by ID
    pub fn get(&self, id: &str) -> Result<Option<Tool>> {
        self.storage.get_tool(id)
    }

    /// Get tool count
    pub fn count(&self) -> Result<usize> {
        self.storage.count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolSource;

    fn create_test_tools() -> Vec<Tool> {
        vec![
            Tool {
                id: "docker-ps".to_string(),
                name: "docker ps".to_string(),
                description: "List running containers".to_string(),
                source: ToolSource::System {
                    path: "/usr/bin/docker".into(),
                },
                updated_at: 1234567890,
            },
            Tool {
                id: "docker-run".to_string(),
                name: "docker run".to_string(),
                description: "Run a command in a new container".to_string(),
                source: ToolSource::System {
                    path: "/usr/bin/docker".into(),
                },
                updated_at: 1234567890,
            },
            Tool {
                id: "git-status".to_string(),
                name: "git status".to_string(),
                description: "Show the working tree status".to_string(),
                source: ToolSource::System {
                    path: "/usr/bin/git".into(),
                },
                updated_at: 1234567890,
            },
            Tool {
                id: "adi.tasks.list".to_string(),
                name: "adi tasks".to_string(),
                description: "Task management with dependency tracking".to_string(),
                source: ToolSource::Plugin {
                    plugin_id: "adi.tasks".to_string(),
                    command: "tasks".to_string(),
                },
                updated_at: 1234567890,
            },
        ]
    }

    #[test]
    fn test_find_exact_match() {
        let search = ToolSearch::open_in_memory().unwrap();

        for tool in create_test_tools() {
            search.storage.upsert_tool(&tool).unwrap();
        }

        let results = search.find("docker ps", 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].tool.id, "docker-ps");
        assert_eq!(results[0].match_type, MatchType::Exact);
    }

    #[test]
    fn test_find_fuzzy_match() {
        let search = ToolSearch::open_in_memory().unwrap();

        for tool in create_test_tools() {
            search.storage.upsert_tool(&tool).unwrap();
        }

        let results = search.find("docker", 10).unwrap();
        assert!(results.len() >= 2);
        // Both docker-ps and docker-run should match
        let ids: Vec<&str> = results.iter().map(|r| r.tool.id.as_str()).collect();
        assert!(ids.contains(&"docker-ps"));
        assert!(ids.contains(&"docker-run"));
    }

    #[test]
    fn test_find_by_description() {
        let search = ToolSearch::open_in_memory().unwrap();

        for tool in create_test_tools() {
            search.storage.upsert_tool(&tool).unwrap();
        }

        let results = search.find("containers", 10).unwrap();
        assert!(!results.is_empty());
        // Should find docker-ps and docker-run
        let ids: Vec<&str> = results.iter().map(|r| r.tool.id.as_str()).collect();
        assert!(ids.contains(&"docker-ps") || ids.contains(&"docker-run"));
    }
}
