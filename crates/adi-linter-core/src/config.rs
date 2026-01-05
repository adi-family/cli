//! Configuration loading and parsing.

use crate::linter::command::{CommandLinter, CommandType, RegexFix};
use crate::linter::external::{ExternalLinter, ExternalLinterConfig};
use crate::registry::{CategoryConfig, LinterRegistry};
use crate::types::{Category, InputMode, LintScope, OutputMode, Severity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Main linter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinterConfig {
    /// Global linter settings.
    #[serde(default)]
    pub linter: GlobalConfig,

    /// Autofix settings.
    #[serde(default)]
    pub autofix: AutofixConfig,

    /// Category configurations.
    #[serde(default)]
    pub categories: HashMap<String, CategoryConfigFile>,

    /// Rules configuration.
    #[serde(default)]
    pub rules: RulesConfig,
}

impl Default for LinterConfig {
    fn default() -> Self {
        Self {
            linter: GlobalConfig::default(),
            autofix: AutofixConfig::default(),
            categories: HashMap::new(),
            rules: RulesConfig::default(),
        }
    }
}

/// Global linter settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Run linters in parallel.
    #[serde(default = "default_true")]
    pub parallel: bool,

    /// Stop on first error.
    #[serde(default)]
    pub fail_fast: bool,

    /// Timeout per linter in seconds.
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Maximum workers for parallel execution.
    #[serde(default)]
    pub max_workers: Option<usize>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            parallel: true,
            fail_fast: false,
            timeout: 30,
            max_workers: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    30
}

/// Autofix configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutofixConfig {
    /// Enable autofix.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum fix iterations.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,

    /// Interactive mode (prompt before each fix).
    #[serde(default)]
    pub interactive: bool,
}

impl Default for AutofixConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_iterations: 10,
            interactive: false,
        }
    }
}

fn default_max_iterations() -> usize {
    10
}

/// Category configuration from file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CategoryConfigFile {
    /// Simple enabled/disabled.
    Simple(bool),
    /// Full configuration.
    Full {
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default)]
        priority: Option<u32>,
        #[serde(default)]
        fail_on: Option<Severity>,
    },
}

impl CategoryConfigFile {
    /// Convert to CategoryConfig.
    pub fn to_config(&self) -> CategoryConfig {
        match self {
            CategoryConfigFile::Simple(enabled) => {
                if *enabled {
                    CategoryConfig::enabled()
                } else {
                    CategoryConfig::disabled()
                }
            }
            CategoryConfigFile::Full {
                enabled,
                priority,
                fail_on,
            } => CategoryConfig {
                enabled: *enabled,
                priority_override: *priority,
                severity_override: None,
                fail_on: *fail_on,
            },
        }
    }
}

/// Rules configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RulesConfig {
    /// External linter rules.
    #[serde(default)]
    pub exec: Vec<ExternalRuleConfig>,

    /// Command linter rules.
    #[serde(default)]
    pub command: Vec<CommandRuleConfig>,

    /// Plugin linter rules.
    #[serde(default)]
    pub plugins: HashMap<String, PluginRuleConfig>,
}

/// External linter rule configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalRuleConfig {
    /// Rule ID.
    pub id: String,

    /// Single category (for backward compatibility).
    #[serde(default)]
    pub category: Option<Category>,

    /// Multiple categories (takes precedence over `category`).
    #[serde(default)]
    pub categories: Vec<Category>,

    /// Command to execute.
    pub exec: String,

    /// Glob patterns.
    #[serde(default)]
    pub glob: GlobPatterns,

    /// Priority override.
    #[serde(default)]
    pub priority: Option<PriorityValue>,

    /// Input mode.
    #[serde(default)]
    pub input: InputMode,

    /// Output mode.
    #[serde(default)]
    pub output: OutputMode,

    /// Timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// Default severity.
    #[serde(default)]
    pub severity: Severity,

    /// Fallback message (for exit code mode).
    #[serde(default)]
    pub message: Option<String>,

    /// Fix command.
    #[serde(default)]
    pub fix: Option<ExternalFixConfig>,
}

/// External fix configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalFixConfig {
    /// Fix command to execute.
    pub exec: String,
}

/// Command linter rule configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRuleConfig {
    /// Rule ID.
    pub id: String,

    /// Single category (for backward compatibility).
    #[serde(default)]
    pub category: Option<Category>,

    /// Multiple categories (takes precedence over `category`).
    #[serde(default)]
    pub categories: Vec<Category>,

    /// Command type and configuration.
    #[serde(flatten)]
    pub command: CommandTypeConfig,

    /// Glob patterns.
    #[serde(default)]
    pub glob: GlobPatterns,

    /// Priority override.
    #[serde(default)]
    pub priority: Option<PriorityValue>,

    /// Severity.
    #[serde(default)]
    pub severity: Severity,

    /// Scope.
    #[serde(default)]
    pub scope: LintScope,

    /// Fix configuration.
    #[serde(default)]
    pub fix: Option<CommandFixConfig>,
}

impl ExternalRuleConfig {
    /// Get resolved categories (prefers `categories` over `category`).
    pub fn resolved_categories(&self) -> Vec<Category> {
        if !self.categories.is_empty() {
            self.categories.clone()
        } else if let Some(cat) = &self.category {
            vec![cat.clone()]
        } else {
            vec![Category::default()]
        }
    }
}

impl CommandRuleConfig {
    /// Get resolved categories (prefers `categories` over `category`).
    pub fn resolved_categories(&self) -> Vec<Category> {
        if !self.categories.is_empty() {
            self.categories.clone()
        } else if let Some(cat) = &self.category {
            vec![cat.clone()]
        } else {
            vec![Category::default()]
        }
    }
}

/// Command type configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CommandTypeConfig {
    RegexForbid {
        pattern: String,
        message: String,
    },
    RegexRequire {
        pattern: String,
        message: String,
    },
    MaxLineLength {
        max: usize,
    },
    MaxFileSize {
        max: usize,
    },
    Contains {
        text: String,
        message: String,
    },
    NotContains {
        text: String,
        message: String,
    },
}

impl CommandTypeConfig {
    /// Convert to CommandType.
    pub fn to_command_type(&self, fix: Option<&CommandFixConfig>) -> CommandType {
        match self {
            CommandTypeConfig::RegexForbid { pattern, message } => CommandType::RegexForbid {
                pattern: pattern.clone(),
                message: message.clone(),
                fix: fix.map(|f| RegexFix {
                    pattern: f.pattern.clone(),
                    replacement: f.replacement.clone(),
                }),
            },
            CommandTypeConfig::RegexRequire { pattern, message } => CommandType::RegexRequire {
                pattern: pattern.clone(),
                message: message.clone(),
            },
            CommandTypeConfig::MaxLineLength { max } => CommandType::MaxLineLength { max: *max },
            CommandTypeConfig::MaxFileSize { max } => CommandType::MaxFileSize { max: *max },
            CommandTypeConfig::Contains { text, message } => CommandType::Contains {
                text: text.clone(),
                message: message.clone(),
            },
            CommandTypeConfig::NotContains { text, message } => CommandType::NotContains {
                text: text.clone(),
                message: message.clone(),
            },
        }
    }
}

/// Command fix configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandFixConfig {
    /// Pattern to match.
    pub pattern: String,
    /// Replacement text.
    pub replacement: String,
}

/// Plugin rule configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginRuleConfig {
    /// Simple enabled/disabled.
    Simple(bool),
    /// Full configuration.
    Full {
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default)]
        category: Option<Category>,
        #[serde(default)]
        priority: Option<u32>,
        #[serde(default)]
        config: Option<serde_json::Value>,
        #[serde(default)]
        rules: HashMap<String, Severity>,
    },
}

/// Glob patterns (single or multiple).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GlobPatterns {
    Single(String),
    Multiple(Vec<String>),
}

impl Default for GlobPatterns {
    fn default() -> Self {
        GlobPatterns::Single("**/*".to_string())
    }
}

impl GlobPatterns {
    /// Convert to vector.
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            GlobPatterns::Single(s) => vec![s.clone()],
            GlobPatterns::Multiple(v) => v.clone(),
        }
    }
}

/// Priority value (name or number).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PriorityValue {
    Name(String),
    Number(u32),
}

impl PriorityValue {
    /// Resolve to a priority number.
    pub fn resolve(&self) -> u32 {
        match self {
            PriorityValue::Number(n) => *n,
            PriorityValue::Name(name) => match name.to_lowercase().as_str() {
                "critical" => crate::types::priority::CRITICAL,
                "high" => crate::types::priority::HIGH,
                "normal" => crate::types::priority::NORMAL,
                "low" => crate::types::priority::LOW,
                "cosmetic" => crate::types::priority::COSMETIC,
                _ => crate::types::priority::NORMAL,
            },
        }
    }
}

impl LinterConfig {
    /// Load configuration from a file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from project directory.
    /// Looks for `.adi/linter.toml` or `linter.toml`.
    pub fn load_from_project(project_path: &Path) -> anyhow::Result<Self> {
        // Try .adi/linter.toml first
        let adi_config = project_path.join(".adi").join("linter.toml");
        if adi_config.exists() {
            return Self::load(&adi_config);
        }

        // Try linter.toml in project root
        let root_config = project_path.join("linter.toml");
        if root_config.exists() {
            return Self::load(&root_config);
        }

        // Return defaults
        Ok(Self::default())
    }

    /// Build a LinterRegistry from this configuration.
    pub fn build_registry(&self) -> anyhow::Result<LinterRegistry> {
        let mut registry = LinterRegistry::new();

        // Configure categories
        for (name, config) in &self.categories {
            let category = parse_category(name);
            registry.configure_category(category, config.to_config());
        }

        // Register external linters
        for rule in &self.rules.exec {
            let external_config = ExternalLinterConfig {
                exec: rule.exec.clone(),
                input_mode: rule.input,
                output_mode: rule.output,
                timeout_secs: rule.timeout,
                severity: rule.severity,
                message: rule.message.clone(),
                fix_exec: rule.fix.as_ref().map(|f| f.exec.clone()),
            };

            let mut linter = ExternalLinter::with_categories(
                &rule.id,
                rule.resolved_categories(),
                rule.glob.to_vec(),
                external_config,
            )?;

            if let Some(priority) = &rule.priority {
                linter = linter.with_priority(priority.resolve());
            }

            registry.register(linter);
        }

        // Register command linters
        for rule in &self.rules.command {
            let command_type = rule.command.to_command_type(rule.fix.as_ref());

            let mut linter = CommandLinter::with_categories(
                &rule.id,
                rule.resolved_categories(),
                rule.glob.to_vec(),
                command_type,
            )?;

            linter = linter.with_scope(rule.scope).with_severity(rule.severity);

            if let Some(priority) = &rule.priority {
                linter = linter.with_priority(priority.resolve());
            }

            registry.register(linter);
        }

        // Plugin linters would be registered separately via plugin system

        Ok(registry)
    }

    /// Get runner config.
    pub fn runner_config(&self, root: &Path) -> crate::runner::RunnerConfig {
        let mut config = crate::runner::RunnerConfig::new(root)
            .parallel(self.linter.parallel)
            .fail_fast(self.linter.fail_fast)
            .timeout(Duration::from_secs(self.linter.timeout));

        if let Some(workers) = self.linter.max_workers {
            config = config.max_workers(workers);
        }

        config
    }

    /// Get autofix config.
    pub fn autofix_config(&self) -> crate::autofix::AutofixConfig {
        crate::autofix::AutofixConfig {
            max_iterations: self.autofix.max_iterations,
            dry_run: false,
            interactive: self.autofix.interactive,
        }
    }
}

fn parse_category(name: &str) -> Category {
    match name.to_lowercase().replace('-', "_").as_str() {
        "architecture" => Category::Architecture,
        "security" => Category::Security,
        "code_quality" | "codequality" => Category::CodeQuality,
        "best_practices" | "bestpractices" => Category::BestPractices,
        "correctness" => Category::Correctness,
        "error_handling" | "errorhandling" => Category::ErrorHandling,
        "performance" => Category::Performance,
        "style" => Category::Style,
        "naming" => Category::Naming,
        "documentation" => Category::Documentation,
        "testing" => Category::Testing,
        _ => Category::Custom(name.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let toml = r#"
[linter]
parallel = true
fail_fast = false
timeout = 60

[categories]
security = { enabled = true, fail_on = "warning" }
style = false

[[rules.command]]
id = "no-todo"
category = "code-quality"
type = "regex-forbid"
pattern = "TODO"
message = "Found TODO"
glob = "**/*.rs"
severity = "warning"
"#;

        let config: LinterConfig = toml::from_str(toml).unwrap();

        assert!(config.linter.parallel);
        assert_eq!(config.linter.timeout, 60);
        assert_eq!(config.rules.command.len(), 1);
        assert_eq!(config.rules.command[0].id, "no-todo");
    }

    #[test]
    fn test_glob_patterns() {
        let single: GlobPatterns = serde_json::from_str(r#""**/*.rs""#).unwrap();
        assert_eq!(single.to_vec(), vec!["**/*.rs".to_string()]);

        let multiple: GlobPatterns =
            serde_json::from_str(r#"["**/*.rs", "**/*.ts"]"#).unwrap();
        assert_eq!(multiple.to_vec().len(), 2);
    }

    #[test]
    fn test_priority_value() {
        let named: PriorityValue = serde_json::from_str(r#""critical""#).unwrap();
        assert_eq!(named.resolve(), 1000);

        let number: PriorityValue = serde_json::from_str("999").unwrap();
        assert_eq!(number.resolve(), 999);
    }
}
