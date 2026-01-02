use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a block in the library
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub String);

impl BlockId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(format!("block_{}", COUNTER.fetch_add(1, Ordering::SeqCst)))
    }
}

impl std::fmt::Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of logical block in the code
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockType {
    // Top-level constructs
    Module,
    Class,
    Function,
    Method,
    AsyncFunction,
    AsyncMethod,
    Arrow,
    Generator,

    // Control flow
    If,
    Else,
    ElseIf,
    Switch,
    Case,
    Default,
    TryCatch,
    Try,
    Catch,
    Finally,

    // Loops
    For,
    ForIn,
    ForOf,
    ForAwait,
    While,
    DoWhile,

    // Expressions/statements
    Call,
    MethodCall,
    AwaitCall,
    New,
    Assignment,
    Destructure,
    Spread,
    Yield,
    YieldFrom,

    // Returns/throws
    Return,
    Throw,
    Break,
    Continue,

    // Declarations
    Variable,
    Const,
    Let,
    Parameter,

    // Class members
    Property,
    Getter,
    Setter,
    Constructor,
    StaticMethod,
    StaticProperty,

    // Imports/exports
    Import,
    Export,
    ExportDefault,

    // Type definitions (for TS)
    Interface,
    TypeAlias,
    Enum,
    EnumMember,

    // Decorators (for TS/Python)
    Decorator,

    // Literals and expressions
    Object,
    Array,
    Template,
    Ternary,
    Binary,
    Unary,
    Member,
    Index,

    // Misc
    Expression,
    Block,
    Statement,
    Unknown,
}

/// Source code location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: String,
    pub start_line: u32,
    pub end_line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_col: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_col: Option<u32>,
}

/// A single logical block in the code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Human-readable name (function name, variable name, expression summary)
    pub name: String,

    /// Type of this block
    #[serde(rename = "type")]
    pub block_type: BlockType,

    /// Variables/data this block consumes (reads)
    pub uses_data: Vec<String>,

    /// Variables/data this block produces (writes/returns)
    pub produces_data: Vec<String>,

    /// Child block IDs (referencing library entries)
    pub children: Vec<BlockId>,

    /// Source code location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,

    /// Raw code snippet (optional, for debugging)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BlockMetadata>,
}

/// Additional metadata for blocks
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlockMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_async: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_static: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_exported: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub decorators: Option<Vec<String>>,

    /// Condition expression for if/while/for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,

    /// Target of import/call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

impl Block {
    pub fn new(name: impl Into<String>, block_type: BlockType) -> Self {
        Self {
            name: name.into(),
            block_type,
            uses_data: Vec::new(),
            produces_data: Vec::new(),
            children: Vec::new(),
            location: None,
            code: None,
            metadata: None,
        }
    }

    pub fn with_location(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_uses(mut self, vars: Vec<String>) -> Self {
        self.uses_data = vars;
        self
    }

    pub fn with_produces(mut self, vars: Vec<String>) -> Self {
        self.produces_data = vars;
        self
    }

    pub fn with_children(mut self, children: Vec<BlockId>) -> Self {
        self.children = children;
        self
    }

    pub fn with_metadata(mut self, metadata: BlockMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn add_child(&mut self, child: BlockId) {
        self.children.push(child);
    }

    pub fn add_uses(&mut self, var: impl Into<String>) {
        let v = var.into();
        if !self.uses_data.contains(&v) {
            self.uses_data.push(v);
        }
    }

    pub fn add_produces(&mut self, var: impl Into<String>) {
        let v = var.into();
        if !self.produces_data.contains(&v) {
            self.produces_data.push(v);
        }
    }
}

/// Flat library of all blocks
pub type BlockLibrary = HashMap<BlockId, Block>;

/// Complete output format for code analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowMapOutput {
    /// Flat map of all blocks by ID
    pub library: BlockLibrary,

    /// Entry points (top-level functions, classes, etc.)
    pub root: Vec<BlockId>,

    /// File path that was analyzed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,

    /// Language detected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

impl FlowMapOutput {
    pub fn new() -> Self {
        Self {
            library: HashMap::new(),
            root: Vec::new(),
            file: None,
            language: None,
        }
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    /// Add a block to the library
    pub fn add_block(&mut self, id: BlockId, block: Block) -> BlockId {
        self.library.insert(id.clone(), block);
        id
    }

    /// Add a block and generate an ID
    pub fn add_block_auto(&mut self, block: Block) -> BlockId {
        let id = BlockId::generate();
        self.add_block(id.clone(), block);
        id
    }

    /// Mark a block as a root entry point
    pub fn add_root(&mut self, id: BlockId) {
        if !self.root.contains(&id) {
            self.root.push(id);
        }
    }

    /// Get a block by ID
    pub fn get_block(&self, id: &BlockId) -> Option<&Block> {
        self.library.get(id)
    }

    /// Get a mutable block by ID
    pub fn get_block_mut(&mut self, id: &BlockId) -> Option<&mut Block> {
        self.library.get_mut(id)
    }

    /// Total block count
    pub fn block_count(&self) -> usize {
        self.library.len()
    }

    /// Merge another output into this one
    pub fn merge(&mut self, other: FlowMapOutput) {
        self.library.extend(other.library);
        self.root.extend(other.root);
    }
}

impl Default for FlowMapOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_creation() {
        let block = Block::new("my_function", BlockType::Function)
            .with_uses(vec!["arg1".to_string(), "arg2".to_string()])
            .with_produces(vec!["result".to_string()]);

        assert_eq!(block.name, "my_function");
        assert_eq!(block.block_type, BlockType::Function);
        assert_eq!(block.uses_data, vec!["arg1", "arg2"]);
        assert_eq!(block.produces_data, vec!["result"]);
    }

    #[test]
    fn test_flowmap_output() {
        let mut output = FlowMapOutput::new()
            .with_file("test.ts")
            .with_language("typescript");

        let func_id = output.add_block_auto(
            Block::new("create_order", BlockType::Function)
                .with_uses(vec!["user_id".to_string(), "items".to_string()])
                .with_produces(vec!["order".to_string()])
        );

        output.add_root(func_id.clone());

        assert_eq!(output.block_count(), 1);
        assert_eq!(output.root.len(), 1);

        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("create_order"));
        assert!(json.contains("user_id"));
    }
}
