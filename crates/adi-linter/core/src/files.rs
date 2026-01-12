//! File iterator - pattern-based file discovery.

use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// File iterator with glob patterns and ignore rules.
pub struct FileIterator {
    root: PathBuf,
    patterns: GlobSet,
    pattern_strings: Vec<String>,
    gitignore: Option<Gitignore>,
    adiignore: Option<Gitignore>,
    follow_symlinks: bool,
    max_depth: Option<usize>,
}

impl FileIterator {
    /// Create a new file iterator rooted at the given path.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            patterns: GlobSet::empty(),
            pattern_strings: Vec::new(),
            gitignore: None,
            adiignore: None,
            follow_symlinks: false,
            max_depth: None,
        }
    }

    /// Add include pattern(s).
    pub fn pattern(mut self, pattern: &str) -> anyhow::Result<Self> {
        self.pattern_strings.push(pattern.to_string());
        self.rebuild_patterns()?;
        Ok(self)
    }

    /// Add multiple patterns at once.
    pub fn patterns(mut self, patterns: &[String]) -> anyhow::Result<Self> {
        self.pattern_strings.extend(patterns.iter().cloned());
        self.rebuild_patterns()?;
        Ok(self)
    }

    fn rebuild_patterns(&mut self) -> anyhow::Result<()> {
        let mut builder = GlobSetBuilder::new();
        for pattern in &self.pattern_strings {
            builder.add(Glob::new(pattern)?);
        }
        self.patterns = builder.build()?;
        Ok(())
    }

    /// Use .gitignore rules.
    pub fn use_gitignore(mut self, enabled: bool) -> Self {
        if enabled {
            self.gitignore = self.load_gitignore();
        } else {
            self.gitignore = None;
        }
        self
    }

    /// Use .adiignore rules.
    pub fn use_adiignore(mut self, enabled: bool) -> Self {
        if enabled {
            self.adiignore = self.load_adiignore();
        } else {
            self.adiignore = None;
        }
        self
    }

    /// Follow symlinks.
    pub fn follow_symlinks(mut self, enabled: bool) -> Self {
        self.follow_symlinks = enabled;
        self
    }

    /// Set max directory depth.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    fn load_gitignore(&self) -> Option<Gitignore> {
        let gitignore_path = self.root.join(".gitignore");
        if !gitignore_path.exists() {
            return None;
        }

        let mut builder = GitignoreBuilder::new(&self.root);
        if builder.add(&gitignore_path).is_some() {
            return None;
        }

        builder.build().ok()
    }

    fn load_adiignore(&self) -> Option<Gitignore> {
        let adiignore_path = self.root.join(".adiignore");
        if !adiignore_path.exists() {
            return None;
        }

        let mut builder = GitignoreBuilder::new(&self.root);
        if builder.add(&adiignore_path).is_some() {
            return None;
        }

        builder.build().ok()
    }

    fn is_ignored(&self, path: &Path) -> bool {
        // Check gitignore
        if let Some(ref gi) = self.gitignore {
            if gi.matched(path, path.is_dir()).is_ignore() {
                return true;
            }
        }

        // Check adiignore
        if let Some(ref ai) = self.adiignore {
            if ai.matched(path, path.is_dir()).is_ignore() {
                return true;
            }
        }

        // Default ignores
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        matches!(
            name,
            ".git" | "node_modules" | "target" | "__pycache__" | ".venv" | "venv"
        )
    }

    fn matches_patterns(&self, path: &Path) -> bool {
        if self.patterns.is_empty() {
            return true; // No patterns = match all
        }
        self.patterns.is_match(path)
    }

    /// Iterate over matching files.
    pub fn iter(&self) -> impl Iterator<Item = PathBuf> + '_ {
        let mut walker = WalkDir::new(&self.root).follow_links(self.follow_symlinks);

        if let Some(depth) = self.max_depth {
            walker = walker.max_depth(depth);
        }

        walker
            .into_iter()
            .filter_entry(|e| !self.is_ignored(e.path()))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .filter(|p| self.matches_patterns(p))
    }

    /// Collect all matching files into a vector.
    pub fn collect(&self) -> Vec<PathBuf> {
        self.iter().collect()
    }

    /// Get the root path.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Builder for creating a FileIterator with common defaults.
pub struct FileIteratorBuilder {
    root: PathBuf,
    patterns: Vec<String>,
    use_gitignore: bool,
    use_adiignore: bool,
    follow_symlinks: bool,
    max_depth: Option<usize>,
}

impl FileIteratorBuilder {
    /// Create a new builder.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            patterns: Vec::new(),
            use_gitignore: true,
            use_adiignore: true,
            follow_symlinks: false,
            max_depth: None,
        }
    }

    /// Add a pattern.
    pub fn pattern(mut self, pattern: impl Into<String>) -> Self {
        self.patterns.push(pattern.into());
        self
    }

    /// Add multiple patterns.
    pub fn patterns(mut self, patterns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.patterns.extend(patterns.into_iter().map(Into::into));
        self
    }

    /// Enable/disable gitignore.
    pub fn use_gitignore(mut self, enabled: bool) -> Self {
        self.use_gitignore = enabled;
        self
    }

    /// Enable/disable adiignore.
    pub fn use_adiignore(mut self, enabled: bool) -> Self {
        self.use_adiignore = enabled;
        self
    }

    /// Follow symlinks.
    pub fn follow_symlinks(mut self, enabled: bool) -> Self {
        self.follow_symlinks = enabled;
        self
    }

    /// Set max depth.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Build the FileIterator.
    pub fn build(self) -> anyhow::Result<FileIterator> {
        let mut iter = FileIterator::new(self.root)
            .use_gitignore(self.use_gitignore)
            .use_adiignore(self.use_adiignore)
            .follow_symlinks(self.follow_symlinks);

        if !self.patterns.is_empty() {
            iter = iter.patterns(&self.patterns)?;
        }

        if let Some(depth) = self.max_depth {
            iter = iter.max_depth(depth);
        }

        Ok(iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_files(dir: &Path) {
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::create_dir_all(dir.join("tests")).unwrap();
        fs::create_dir_all(dir.join("node_modules")).unwrap();

        fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(dir.join("src/lib.rs"), "pub fn foo() {}").unwrap();
        fs::write(dir.join("tests/test.rs"), "#[test] fn test() {}").unwrap();
        fs::write(dir.join("README.md"), "# Readme").unwrap();
        fs::write(dir.join("node_modules/pkg.js"), "// ignored").unwrap();
    }

    #[test]
    fn test_file_iterator_all_files() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path());

        let iter = FileIterator::new(dir.path());
        let files: Vec<_> = iter.iter().collect();

        // Should find 4 files (excluding node_modules)
        assert_eq!(files.len(), 4);
    }

    #[test]
    fn test_file_iterator_with_pattern() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path());

        let iter = FileIterator::new(dir.path()).pattern("**/*.rs").unwrap();
        let files: Vec<_> = iter.iter().collect();

        // Should find 3 .rs files
        assert_eq!(files.len(), 3);
        assert!(files.iter().all(|f| f.extension().unwrap() == "rs"));
    }

    #[test]
    fn test_file_iterator_ignores_node_modules() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path());

        let iter = FileIterator::new(dir.path());
        let files: Vec<_> = iter.iter().collect();

        assert!(
            !files
                .iter()
                .any(|f| f.to_string_lossy().contains("node_modules"))
        );
    }

    #[test]
    fn test_file_iterator_builder() {
        let dir = TempDir::new().unwrap();
        create_test_files(dir.path());

        let iter = FileIteratorBuilder::new(dir.path())
            .pattern("**/*.rs")
            .pattern("**/*.md")
            .use_gitignore(true)
            .build()
            .unwrap();

        let files: Vec<_> = iter.iter().collect();
        assert_eq!(files.len(), 4); // 3 .rs + 1 .md
    }
}
