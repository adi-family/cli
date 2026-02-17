//! Configuration loading and parsing.
//!
//! Configuration is loaded from `.adi/linters/` directory:
//! - `config.toml` - Global linter settings and category configuration
//! - `<rule-name>.toml` - Individual rule files (one per linter rule)
//! - `<rule-name>.toml.example` - Example files (ignored)

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
    RegexForbid { pattern: String, message: String },
    RegexRequire { pattern: String, message: String },
    MaxLineLength { max: usize },
    MaxFileSize { max: usize },
    MaxFunctionLength { max: usize },
    Contains { text: String, message: String },
    NotContains { text: String, message: String },
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
            CommandTypeConfig::MaxFunctionLength { max } => {
                CommandType::MaxFunctionLength { max: *max }
            }
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

// === New Format: Individual Rule Files ===

/// Global linter configuration (from config.toml in linters directory).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalLinterConfig {
    #[serde(default)]
    pub linter: GlobalConfig,
    #[serde(default)]
    pub autofix: AutofixConfig,
    #[serde(default)]
    pub categories: HashMap<String, CategoryConfigFile>,
}

/// Individual rule file configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualRuleConfig {
    pub rule: RuleDefinition,
}

/// Rule definition within an individual rule file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDefinition {
    /// Rule ID.
    pub id: String,
    /// Rule type: "exec" or "command".
    #[serde(rename = "type")]
    pub rule_type: RuleType,
    /// Single category.
    #[serde(default)]
    pub category: Option<Category>,
    /// Multiple categories.
    #[serde(default)]
    pub categories: Vec<Category>,
    /// Severity.
    #[serde(default)]
    pub severity: Severity,
    /// Priority override.
    #[serde(default)]
    pub priority: Option<PriorityValue>,
    /// Glob configuration.
    #[serde(default)]
    pub glob: GlobConfig,
    /// Command configuration (for command type).
    #[serde(default)]
    pub command: Option<CommandConfig>,
    /// Exec configuration (for exec type).
    #[serde(default)]
    pub exec: Option<ExecConfig>,
    /// Fix configuration.
    #[serde(default)]
    pub fix: Option<FixConfig>,
    /// Scope (for command type).
    #[serde(default)]
    pub scope: LintScope,
}

/// Rule type enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RuleType {
    Exec,
    Command,
}

/// Glob configuration for rule files.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobConfig {
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl GlobConfig {
    pub fn to_patterns(&self) -> Vec<String> {
        if self.patterns.is_empty() {
            vec!["**/*".to_string()]
        } else {
            self.patterns.clone()
        }
    }
}

/// Command configuration for rule files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CommandConfig {
    RegexForbid { pattern: String, message: String },
    RegexRequire { pattern: String, message: String },
    MaxLineLength { max: usize },
    MaxFileSize { max: usize },
    MaxFunctionLength { max: usize },
    Contains { text: String, message: String },
    NotContains { text: String, message: String },
}

/// Exec configuration for rule files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecConfig {
    pub command: String,
    #[serde(default)]
    pub output: OutputMode,
    #[serde(default)]
    pub input: InputMode,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default)]
    pub message: Option<String>,
}

/// Fix configuration for rule files.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FixConfig {
    /// Regex-based fix (for command rules).
    Regex {
        pattern: String,
        replacement: String,
    },
    /// Exec-based fix (for exec rules).
    Exec { command: String },
}

/// Enum for parsed rule files.
pub enum LinterRuleFile {
    Exec(ExternalRuleConfig),
    Command(CommandRuleConfig),
}

impl IndividualRuleConfig {
    /// Convert to internal rule representation.
    pub fn into_rule(self) -> anyhow::Result<LinterRuleFile> {
        let rule = self.rule;

        match rule.rule_type {
            RuleType::Exec => {
                let exec = rule.exec.ok_or_else(|| {
                    anyhow::anyhow!(
                        "Rule '{}' is type 'exec' but missing [rule.exec] section",
                        rule.id
                    )
                })?;

                Ok(LinterRuleFile::Exec(ExternalRuleConfig {
                    id: rule.id,
                    category: rule.category,
                    categories: rule.categories,
                    exec: exec.command,
                    glob: GlobPatterns::Multiple(rule.glob.to_patterns()),
                    priority: rule.priority,
                    input: exec.input,
                    output: exec.output,
                    timeout: exec.timeout,
                    severity: rule.severity,
                    message: exec.message,
                    fix: match rule.fix {
                        Some(FixConfig::Exec { command }) => {
                            Some(ExternalFixConfig { exec: command })
                        }
                        _ => None,
                    },
                }))
            }
            RuleType::Command => {
                let cmd = rule.command.ok_or_else(|| {
                    anyhow::anyhow!(
                        "Rule '{}' is type 'command' but missing [rule.command] section",
                        rule.id
                    )
                })?;

                let command_type = match cmd {
                    CommandConfig::RegexForbid { pattern, message } => {
                        CommandTypeConfig::RegexForbid { pattern, message }
                    }
                    CommandConfig::RegexRequire { pattern, message } => {
                        CommandTypeConfig::RegexRequire { pattern, message }
                    }
                    CommandConfig::MaxLineLength { max } => {
                        CommandTypeConfig::MaxLineLength { max }
                    }
                    CommandConfig::MaxFileSize { max } => CommandTypeConfig::MaxFileSize { max },
                    CommandConfig::MaxFunctionLength { max } => {
                        CommandTypeConfig::MaxFunctionLength { max }
                    }
                    CommandConfig::Contains { text, message } => {
                        CommandTypeConfig::Contains { text, message }
                    }
                    CommandConfig::NotContains { text, message } => {
                        CommandTypeConfig::NotContains { text, message }
                    }
                };

                let fix = match rule.fix {
                    Some(FixConfig::Regex {
                        pattern,
                        replacement,
                    }) => Some(CommandFixConfig {
                        pattern,
                        replacement,
                    }),
                    _ => None,
                };

                Ok(LinterRuleFile::Command(CommandRuleConfig {
                    id: rule.id,
                    category: rule.category,
                    categories: rule.categories,
                    command: command_type,
                    glob: GlobPatterns::Multiple(rule.glob.to_patterns()),
                    priority: rule.priority,
                    severity: rule.severity,
                    scope: rule.scope,
                    fix,
                }))
            }
        }
    }
}

impl LinterConfig {
    /// Load configuration from project directory.
    ///
    /// Looks for `.adi/linters/` directory with `config.toml` and individual rule files.
    pub fn load_from_project(project_path: &Path) -> anyhow::Result<Self> {
        let linters_dir = project_path.join(".adi").join("linters");
        if linters_dir.exists() && linters_dir.is_dir() {
            return Self::load_from_linters_dir(&linters_dir);
        }

        // Return defaults if no linters directory
        Ok(Self::default())
    }

    /// Load configuration from a linters directory.
    ///
    /// Reads `config.toml` for global settings and individual `.toml` files for rules.
    fn load_from_linters_dir(linters_dir: &Path) -> anyhow::Result<Self> {
        let mut config = Self::default();

        // Load global config from config.toml
        let config_path = linters_dir.join("config.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let global_config: GlobalLinterConfig = toml::from_str(&content)?;
            config.linter = global_config.linter;
            config.autofix = global_config.autofix;
            config.categories = global_config.categories;
        }

        // Load individual rule files
        for entry in std::fs::read_dir(linters_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip non-toml files, config.toml, and .example files
            if !path.is_file() {
                continue;
            }
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if file_name == "config.toml"
                || !file_name.ends_with(".toml")
                || file_name.ends_with(".example")
            {
                continue;
            }

            // Load the rule file
            match Self::load_rule_file(&path) {
                Ok(rule) => match rule {
                    LinterRuleFile::Exec(exec_rule) => {
                        config.rules.exec.push(exec_rule);
                    }
                    LinterRuleFile::Command(cmd_rule) => {
                        config.rules.command.push(cmd_rule);
                    }
                },
                Err(e) => {
                    // Log warning but continue loading other rules
                    eprintln!(
                        "Warning: Failed to load linter rule from {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        Ok(config)
    }

    /// Load a single rule file.
    fn load_rule_file(path: &Path) -> anyhow::Result<LinterRuleFile> {
        let content = std::fs::read_to_string(path)?;
        let rule_config: IndividualRuleConfig = toml::from_str(&content)?;
        rule_config.into_rule()
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
    fn test_parse_global_config() {
        let toml = r#"
[linter]
parallel = true
fail_fast = false
timeout = 60

[autofix]
enabled = true
max_iterations = 5

[categories]
security = { enabled = true, fail_on = "warning" }
style = false
"#;

        let config: GlobalLinterConfig = toml::from_str(toml).unwrap();

        assert!(config.linter.parallel);
        assert_eq!(config.linter.timeout, 60);
        assert_eq!(config.autofix.max_iterations, 5);
    }

    #[test]
    fn test_parse_command_rule() {
        let toml = r#"
[rule]
id = "no-todo"
type = "command"
category = "code-quality"
severity = "warning"

[rule.command]
type = "regex-forbid"
pattern = "TODO|FIXME"
message = "Found TODO"

[rule.glob]
patterns = ["**/*.rs", "**/*.ts"]
"#;

        let rule_config: IndividualRuleConfig = toml::from_str(toml).unwrap();
        let rule = rule_config.into_rule().unwrap();

        match rule {
            LinterRuleFile::Command(cmd) => {
                assert_eq!(cmd.id, "no-todo");
                assert_eq!(cmd.glob.to_vec().len(), 2);
            }
            _ => panic!("Expected command rule"),
        }
    }

    #[test]
    fn test_parse_exec_rule() {
        let toml = r#"
[rule]
id = "shellcheck"
type = "exec"
category = "correctness"
severity = "warning"

[rule.exec]
command = "shellcheck -f json {file}"
output = "json"
timeout = 30

[rule.glob]
patterns = ["**/*.sh"]
"#;

        let rule_config: IndividualRuleConfig = toml::from_str(toml).unwrap();
        let rule = rule_config.into_rule().unwrap();

        match rule {
            LinterRuleFile::Exec(exec) => {
                assert_eq!(exec.id, "shellcheck");
                assert_eq!(exec.exec, "shellcheck -f json {file}");
            }
            _ => panic!("Expected exec rule"),
        }
    }

    #[test]
    fn test_glob_patterns() {
        let single: GlobPatterns = serde_json::from_str(r#""**/*.rs""#).unwrap();
        assert_eq!(single.to_vec(), vec!["**/*.rs".to_string()]);

        let multiple: GlobPatterns = serde_json::from_str(r#"["**/*.rs", "**/*.ts"]"#).unwrap();
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
