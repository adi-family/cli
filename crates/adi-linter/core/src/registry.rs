//! Linter registry - manages linters with priority and category grouping.

use crate::linter::Linter;
use crate::types::{Category, Severity};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;

/// Configuration for a category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryConfig {
    /// Whether this category is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Priority override for all linters in this category.
    #[serde(default)]
    pub priority_override: Option<u32>,
    /// Severity override for all diagnostics from this category.
    #[serde(default)]
    pub severity_override: Option<Severity>,
    /// Fail if any diagnostic >= this severity.
    #[serde(default)]
    pub fail_on: Option<Severity>,
}

fn default_true() -> bool {
    true
}

impl Default for CategoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            priority_override: None,
            severity_override: None,
            fail_on: None,
        }
    }
}

impl CategoryConfig {
    /// Create an enabled category config.
    pub fn enabled() -> Self {
        Self::default()
    }

    /// Create a disabled category config.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Set priority override.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority_override = Some(priority);
        self
    }

    /// Set fail_on severity.
    pub fn with_fail_on(mut self, severity: Severity) -> Self {
        self.fail_on = Some(severity);
        self
    }
}

/// Registry of linters with priority and category management.
pub struct LinterRegistry {
    linters: Vec<Arc<dyn Linter>>,
    category_config: HashMap<Category, CategoryConfig>,
}

impl LinterRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            linters: Vec::new(),
            category_config: HashMap::new(),
        }
    }

    /// Register a linter.
    pub fn register<L: Linter + 'static>(&mut self, linter: L) {
        self.linters.push(Arc::new(linter));
    }

    /// Register a linter (Arc version).
    pub fn register_arc(&mut self, linter: Arc<dyn Linter>) {
        self.linters.push(linter);
    }

    /// Configure a category.
    pub fn configure_category(&mut self, category: Category, config: CategoryConfig) {
        self.category_config.insert(category, config);
    }

    /// Get category config (or default if not configured).
    pub fn category_config(&self, category: &Category) -> CategoryConfig {
        self.category_config
            .get(category)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if a category is enabled.
    pub fn is_category_enabled(&self, category: &Category) -> bool {
        self.category_config(category).enabled
    }

    /// Get all registered linters.
    pub fn all_linters(&self) -> impl Iterator<Item = &Arc<dyn Linter>> {
        self.linters.iter()
    }

    /// Get active linters (from enabled categories).
    /// A linter is active if at least one of its categories is enabled.
    pub fn active_linters(&self) -> impl Iterator<Item = &Arc<dyn Linter>> {
        self.linters
            .iter()
            .filter(|l| l.categories().iter().any(|c| self.is_category_enabled(c)))
    }

    /// Get linters by category.
    /// Returns linters that have the specified category (among their categories).
    pub fn by_category(&self, category: &Category) -> Vec<&Arc<dyn Linter>> {
        self.linters
            .iter()
            .filter(|l| l.has_category(category))
            .collect()
    }

    /// Get linters sorted by priority (descending).
    pub fn by_priority(&self) -> Vec<&Arc<dyn Linter>> {
        let mut linters: Vec<_> = self.active_linters().collect();
        linters.sort_by(|a, b| {
            self.effective_priority(b.as_ref())
                .cmp(&self.effective_priority(a.as_ref()))
        });
        linters
    }

    /// Group linters by priority level.
    pub fn by_priority_groups(&self) -> BTreeMap<u32, Vec<Arc<dyn Linter>>> {
        let mut groups: BTreeMap<u32, Vec<Arc<dyn Linter>>> = BTreeMap::new();

        for linter in self.active_linters() {
            let priority = self.effective_priority(linter.as_ref());
            groups
                .entry(priority)
                .or_default()
                .push(Arc::clone(linter));
        }

        groups
    }

    /// Group by category, then by priority within each category.
    /// A linter with multiple categories appears in each category group.
    pub fn by_category_and_priority(&self) -> HashMap<Category, BTreeMap<u32, Vec<Arc<dyn Linter>>>> {
        let mut result: HashMap<Category, BTreeMap<u32, Vec<Arc<dyn Linter>>>> = HashMap::new();

        for linter in self.active_linters() {
            let priority = self.effective_priority(linter.as_ref());

            // Add linter to each of its categories
            for category in linter.categories() {
                result
                    .entry(category.clone())
                    .or_default()
                    .entry(priority)
                    .or_default()
                    .push(Arc::clone(linter));
            }
        }

        result
    }

    /// Get linters that match a specific file.
    pub fn for_file(&self, path: &Path) -> Vec<&Arc<dyn Linter>> {
        self.active_linters()
            .filter(|l| l.matches(path))
            .collect()
    }

    /// Get linter by ID.
    pub fn get(&self, id: &str) -> Option<&Arc<dyn Linter>> {
        self.linters.iter().find(|l| l.id() == id)
    }

    /// Get effective priority for a linter.
    /// Priority order: linter priority > highest category override > category default
    /// For multi-category linters, uses the highest priority among all categories.
    pub fn effective_priority(&self, linter: &dyn Linter) -> u32 {
        // Check category overrides - use highest priority override
        let category_override = linter
            .categories()
            .iter()
            .filter_map(|cat| {
                self.category_config
                    .get(cat)
                    .and_then(|config| config.priority_override)
            })
            .max();

        if let Some(override_priority) = category_override {
            return override_priority;
        }

        // Use linter's priority (which may be category default or explicit)
        linter.priority()
    }

    /// Get number of registered linters.
    pub fn len(&self) -> usize {
        self.linters.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.linters.is_empty()
    }

    /// Get number of active linters.
    pub fn active_count(&self) -> usize {
        self.active_linters().count()
    }

    /// Get all unique categories used by registered linters.
    pub fn categories(&self) -> Vec<Category> {
        let mut categories: Vec<_> = self
            .linters
            .iter()
            .flat_map(|l| l.categories().iter().cloned())
            .collect();
        categories.sort_by(|a, b| a.display_name().cmp(b.display_name()));
        categories.dedup();
        categories
    }
}

impl Default for LinterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a LinterRegistry.
pub struct LinterRegistryBuilder {
    registry: LinterRegistry,
}

impl LinterRegistryBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            registry: LinterRegistry::new(),
        }
    }

    /// Add a linter.
    pub fn linter<L: Linter + 'static>(mut self, linter: L) -> Self {
        self.registry.register(linter);
        self
    }

    /// Configure a category.
    pub fn category(mut self, category: Category, config: CategoryConfig) -> Self {
        self.registry.configure_category(category, config);
        self
    }

    /// Disable a category.
    pub fn disable_category(mut self, category: Category) -> Self {
        self.registry
            .configure_category(category, CategoryConfig::disabled());
        self
    }

    /// Build the registry.
    pub fn build(self) -> LinterRegistry {
        self.registry
    }
}

impl Default for LinterRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::command::{CommandLinter, CommandType};

    fn create_test_linter(id: &str, category: Category) -> CommandLinter {
        CommandLinter::new(
            id,
            category,
            vec!["**/*".to_string()],
            CommandType::MaxLineLength { max: 100 },
        )
        .unwrap()
    }

    #[test]
    fn test_registry_basic() {
        let mut registry = LinterRegistry::new();
        registry.register(create_test_linter("lint1", Category::Security));
        registry.register(create_test_linter("lint2", Category::CodeQuality));

        assert_eq!(registry.len(), 2);
        assert!(registry.get("lint1").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_category_disable() {
        let mut registry = LinterRegistry::new();
        registry.register(create_test_linter("sec", Category::Security));
        registry.register(create_test_linter("qual", Category::CodeQuality));

        // Both active initially
        assert_eq!(registry.active_count(), 2);

        // Disable security
        registry.configure_category(Category::Security, CategoryConfig::disabled());
        assert_eq!(registry.active_count(), 1);

        // Only quality linter active
        let active: Vec<_> = registry.active_linters().collect();
        assert_eq!(active[0].id(), "qual");
    }

    #[test]
    fn test_priority_groups() {
        let mut registry = LinterRegistry::new();
        registry.register(create_test_linter("sec1", Category::Security));
        registry.register(create_test_linter("sec2", Category::Security));
        registry.register(create_test_linter("style", Category::Style));

        let groups = registry.by_priority_groups();

        // Security has priority 1000, Style has priority 100
        assert!(groups.contains_key(&1000));
        assert!(groups.contains_key(&100));
        assert_eq!(groups[&1000].len(), 2); // 2 security linters
        assert_eq!(groups[&100].len(), 1); // 1 style linter
    }

    #[test]
    fn test_priority_override() {
        let mut registry = LinterRegistry::new();
        registry.register(create_test_linter("style", Category::Style));

        // Default style priority is 100
        {
            let linter = registry.get("style").unwrap();
            assert_eq!(registry.effective_priority(linter.as_ref()), 100);
        }

        // Override style category priority
        registry.configure_category(Category::Style, CategoryConfig::enabled().with_priority(999));

        {
            let linter = registry.get("style").unwrap();
            assert_eq!(registry.effective_priority(linter.as_ref()), 999);
        }
    }

    #[test]
    fn test_builder() {
        let registry = LinterRegistryBuilder::new()
            .linter(create_test_linter("lint1", Category::Security))
            .linter(create_test_linter("lint2", Category::Style))
            .disable_category(Category::Style)
            .build();

        assert_eq!(registry.len(), 2);
        assert_eq!(registry.active_count(), 1);
    }
}
