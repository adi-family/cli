//! JSON tree view state management - framework-agnostic
//!
//! Provides collapsible JSON tree state and rendering helpers.
//! Works with any UI framework - returns data structures for rendering.
//!
//! # Generic JSON Support
//!
//! This crate is generic over JSON implementations. While it defaults to `serde_json::Value`,
//! you can use it with any type that implements the [`JsonValue`] trait.
//!
//! ```
//! use lib_json_tree::{JsonValue, flatten_json, JsonTreeState};
//! use serde_json::json;
//!
//! let json = json!({"name": "Alice", "age": 30});
//! let state = JsonTreeState::new();
//! let nodes = flatten_json(&json, &state);
//! assert_eq!(nodes.len(), 3); // root object + 2 fields
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::HashSet;

// Generic JSON value abstraction
mod value;
pub use value::{JsonArray, JsonObject, JsonValue};

// ==================== SmallVec Type Aliases ====================

/// SmallVec type for small JSON trees (avoids heap allocation for <= 32 nodes)
///
/// Use this type when you expect small JSON structures and want to avoid
/// heap allocation overhead.
///
/// The lifetime `'a` is tied to the JSON value being flattened.
pub type SmallNodeVec<'a> = SmallVec<[JsonTreeNode<'a>; 32]>;

/// Path segment for building JSON paths
#[derive(Debug, Clone)]
pub enum PathSegment<'a> {
    /// Object key (e.g., "name" -> ".name" or "name" at root)
    Key(&'a str),
    /// Array index (e.g., 0 -> "[0]")
    Index(usize),
}

// ==================== Type-Safe Path Builder ====================

/// Type-safe JSON path builder
///
/// Provides a fluent API for building JSON paths without string concatenation.
///
/// # Examples
/// ```
/// use lib_json_tree::JsonPath;
///
/// let path = JsonPath::root().key("users").index(0).key("name");
/// assert_eq!(path.to_string(), "users[0].name");
///
/// let empty = JsonPath::root();
/// assert_eq!(empty.to_string(), "");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JsonPath {
    segments: Vec<OwnedPathSegment>,
}

/// Owned path segment for storage in JsonPath
#[derive(Debug, Clone, PartialEq, Eq)]
enum OwnedPathSegment {
    Key(String),
    Index(usize),
}

impl JsonPath {
    /// Create a new empty path (root)
    pub fn root() -> Self {
        Self { segments: vec![] }
    }

    /// Parse a dot-notation path string into a JsonPath
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::JsonPath;
    ///
    /// let path = JsonPath::parse("users[0].name");
    /// assert_eq!(path.to_string(), "users[0].name");
    ///
    /// let path = JsonPath::parse("[0][1]");
    /// assert_eq!(path.to_string(), "[0][1]");
    /// ```
    pub fn parse(path: &str) -> Self {
        if path.is_empty() {
            return Self::root();
        }

        let mut result = Self::root();
        let mut chars = path.chars().peekable();
        let mut current_key = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '.' => {
                    if !current_key.is_empty() {
                        result.segments.push(OwnedPathSegment::Key(current_key));
                        current_key = String::new();
                    }
                }
                '[' => {
                    if !current_key.is_empty() {
                        result.segments.push(OwnedPathSegment::Key(current_key));
                        current_key = String::new();
                    }
                    let mut index_str = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == ']' {
                            chars.next();
                            break;
                        }
                        index_str.push(chars.next().unwrap());
                    }
                    if let Ok(idx) = index_str.parse::<usize>() {
                        result.segments.push(OwnedPathSegment::Index(idx));
                    }
                }
                ']' => {}
                _ => {
                    current_key.push(ch);
                }
            }
        }

        if !current_key.is_empty() {
            result.segments.push(OwnedPathSegment::Key(current_key));
        }

        result
    }

    /// Add a key segment to the path
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::JsonPath;
    ///
    /// let path = JsonPath::root().key("users").key("name");
    /// assert_eq!(path.to_string(), "users.name");
    /// ```
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.segments.push(OwnedPathSegment::Key(key.into()));
        self
    }

    /// Add an index segment to the path
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::JsonPath;
    ///
    /// let path = JsonPath::root().key("items").index(0);
    /// assert_eq!(path.to_string(), "items[0]");
    /// ```
    pub fn index(mut self, idx: usize) -> Self {
        self.segments.push(OwnedPathSegment::Index(idx));
        self
    }

    /// Get the number of segments in the path
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Check if the path is empty (root)
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Get the parent path (all segments except the last)
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::JsonPath;
    ///
    /// let path = JsonPath::root().key("users").index(0).key("name");
    /// let parent = path.parent();
    /// assert_eq!(parent.map(|p| p.to_string()), Some("users[0]".to_string()));
    ///
    /// let root = JsonPath::root();
    /// assert!(root.parent().is_none());
    /// ```
    pub fn parent(&self) -> Option<Self> {
        if self.segments.is_empty() {
            None
        } else {
            Some(Self {
                segments: self.segments[..self.segments.len() - 1].to_vec(),
            })
        }
    }

    /// Convert to JSON Pointer format (RFC 6901)
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::JsonPath;
    ///
    /// let path = JsonPath::root().key("users").index(0).key("name");
    /// assert_eq!(path.to_json_pointer(), "/users/0/name");
    /// ```
    pub fn to_json_pointer(&self) -> String {
        path_to_pointer(&self.to_string())
    }

    /// Convert to jq path format
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::JsonPath;
    ///
    /// let path = JsonPath::root().key("users").index(0).key("name");
    /// assert_eq!(path.to_jq_path(), ".users[0].name");
    /// ```
    pub fn to_jq_path(&self) -> String {
        let s = self.to_string();
        if s.is_empty() {
            ".".to_string()
        } else {
            format!(".{}", s)
        }
    }

    /// Convert to bracket notation
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::JsonPath;
    ///
    /// let path = JsonPath::root().key("users").index(0).key("name");
    /// assert_eq!(path.to_bracket_notation(), r#"["users"][0]["name"]"#);
    /// ```
    pub fn to_bracket_notation(&self) -> String {
        let mut result = String::new();
        for segment in &self.segments {
            match segment {
                OwnedPathSegment::Key(k) => {
                    result.push_str(&format!("[\"{}\"]", k));
                }
                OwnedPathSegment::Index(i) => {
                    result.push_str(&format!("[{}]", i));
                }
            }
        }
        result
    }

    /// Get a value from JSON using this path
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::JsonPath;
    /// use serde_json::json;
    ///
    /// let json = json!({"users": [{"name": "Alice"}]});
    /// let path = JsonPath::root().key("users").index(0).key("name");
    /// assert_eq!(path.get(&json), Some(&json!("Alice")));
    /// ```
    pub fn get<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        let mut current = value;
        for segment in &self.segments {
            current = match segment {
                OwnedPathSegment::Key(k) => current.get(k)?,
                OwnedPathSegment::Index(i) => current.get(i)?,
            };
        }
        Some(current)
    }
}

impl std::fmt::Display for JsonPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for segment in &self.segments {
            match segment {
                OwnedPathSegment::Key(k) => {
                    if !first {
                        write!(f, ".")?;
                    }
                    write!(f, "{}", k)?;
                }
                OwnedPathSegment::Index(i) => {
                    write!(f, "[{}]", i)?;
                }
            }
            first = false;
        }
        Ok(())
    }
}

impl From<&str> for JsonPath {
    fn from(s: &str) -> Self {
        Self::parse(s)
    }
}

impl From<String> for JsonPath {
    fn from(s: String) -> Self {
        Self::parse(&s)
    }
}

/// Build a child path from parent path and segment
///
/// # Examples
/// ```
/// use lib_json_tree::{build_child_path, PathSegment};
///
/// assert_eq!(build_child_path("", PathSegment::Key("users")), "users");
/// assert_eq!(build_child_path("users", PathSegment::Index(0)), "users[0]");
/// assert_eq!(build_child_path("users[0]", PathSegment::Key("name")), "users[0].name");
/// ```
pub fn build_child_path(parent: &str, segment: PathSegment) -> String {
    match segment {
        PathSegment::Key(k) => {
            if parent.is_empty() {
                k.to_string()
            } else {
                format!("{}.{}", parent, k)
            }
        }
        PathSegment::Index(i) => format!("{}[{}]", parent, i),
    }
}

/// Unique ID for a JSON node (path string)
pub type JsonNodeId = String;

/// JSON tree view state - supports serialization for persistence
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JsonTreeState {
    /// Set of collapsed node paths
    pub collapsed: HashSet<JsonNodeId>,
}

impl JsonTreeState {
    pub fn new() -> Self {
        Self {
            collapsed: HashSet::new(),
        }
    }

    /// Create a builder for constructing JsonTreeState with various options
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::JsonTreeState;
    /// use serde_json::json;
    ///
    /// let json = json!({"a": {"b": 1}, "c": 2});
    /// let state = JsonTreeState::builder()
    ///     .collapsed_at_depth(&json, 1)
    ///     .build();
    /// assert!(state.is_collapsed("a"));
    /// ```
    pub fn builder() -> JsonTreeStateBuilder {
        JsonTreeStateBuilder::new()
    }

    /// Toggle collapse state for a node
    pub fn toggle(&mut self, path: &str) {
        if self.collapsed.contains(path) {
            self.collapsed.remove(path);
        } else {
            self.collapsed.insert(path.to_string());
        }
    }

    /// Toggle collapse state for a node using JsonPath
    pub fn toggle_path(&mut self, path: &JsonPath) {
        self.toggle(&path.to_string());
    }

    /// Check if a node is collapsed
    pub fn is_collapsed(&self, path: &str) -> bool {
        self.collapsed.contains(path)
    }

    /// Check if a node is collapsed using JsonPath
    pub fn is_collapsed_path(&self, path: &JsonPath) -> bool {
        self.is_collapsed(&path.to_string())
    }

    /// Expand a node
    pub fn expand(&mut self, path: &str) {
        self.collapsed.remove(path);
    }

    /// Expand a node using JsonPath
    pub fn expand_path(&mut self, path: &JsonPath) {
        self.expand(&path.to_string());
    }

    /// Collapse a node
    pub fn collapse(&mut self, path: &str) {
        self.collapsed.insert(path.to_string());
    }

    /// Collapse a node using JsonPath
    pub fn collapse_path(&mut self, path: &JsonPath) {
        self.collapse(&path.to_string());
    }

    /// Expand all nodes
    pub fn expand_all(&mut self) {
        self.collapsed.clear();
    }

    /// Collapse all nodes at depth 2 or deeper (auto-collapse for large JSON)
    ///
    /// Works with any type implementing [`JsonValue`].
    pub fn collapse_deep<V: JsonValue>(&mut self, json: &V, max_depth: usize) {
        fn collect_paths<V: JsonValue>(
            value: &V,
            path: String,
            depth: usize,
            max_depth: usize,
            paths: &mut Vec<String>,
        ) {
            use crate::{build_child_path, PathSegment};

            if depth >= max_depth {
                if value.is_object() || value.is_array() {
                    paths.push(path);
                }
                return;
            }

            if let Some(obj) = value.as_object() {
                for (key, val) in obj.iter() {
                    let child_path = build_child_path(&path, PathSegment::Key(key));
                    collect_paths(val, child_path, depth + 1, max_depth, paths);
                }
            } else if let Some(arr) = value.as_array() {
                for (idx, val) in arr.iter().enumerate() {
                    let child_path = build_child_path(&path, PathSegment::Index(idx));
                    collect_paths(val, child_path, depth + 1, max_depth, paths);
                }
            }
        }

        let mut paths = Vec::new();
        collect_paths(json, String::new(), 0, max_depth, &mut paths);
        for path in paths {
            self.collapsed.insert(path);
        }
    }
}

// ==================== JsonTreeState Builder ====================

/// Builder for constructing JsonTreeState with various options
///
/// # Examples
/// ```
/// use lib_json_tree::JsonTreeStateBuilder;
/// use serde_json::json;
///
/// let json = json!({"a": {"b": 1}, "c": 2});
///
/// // Simple builder usage
/// let state = JsonTreeStateBuilder::new()
///     .collapsed_at_depth(&json, 1)
///     .expand_path("a")
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct JsonTreeStateBuilder {
    collapsed: HashSet<JsonNodeId>,
}

impl JsonTreeStateBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            collapsed: HashSet::new(),
        }
    }

    /// Collapse all nodes at or beyond a certain depth
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::JsonTreeStateBuilder;
    /// use serde_json::json;
    ///
    /// let json = json!({"a": {"b": 1}, "c": 2});
    /// let state = JsonTreeStateBuilder::new()
    ///     .collapsed_at_depth(&json, 1)
    ///     .build();
    /// assert!(state.is_collapsed("a"));
    /// assert!(!state.is_collapsed("c")); // c is a primitive at depth 1
    /// ```
    pub fn collapsed_at_depth<V: JsonValue>(mut self, json: &V, max_depth: usize) -> Self {
        fn collect_paths<V: JsonValue>(
            value: &V,
            path: String,
            depth: usize,
            max_depth: usize,
            paths: &mut Vec<String>,
        ) {
            use crate::{build_child_path, PathSegment};

            if depth >= max_depth {
                if value.is_object() || value.is_array() {
                    paths.push(path);
                }
                return;
            }

            if let Some(obj) = value.as_object() {
                for (key, val) in obj.iter() {
                    let child_path = build_child_path(&path, PathSegment::Key(key));
                    collect_paths(val, child_path, depth + 1, max_depth, paths);
                }
            } else if let Some(arr) = value.as_array() {
                for (idx, val) in arr.iter().enumerate() {
                    let child_path = build_child_path(&path, PathSegment::Index(idx));
                    collect_paths(val, child_path, depth + 1, max_depth, paths);
                }
            }
        }

        let mut paths = Vec::new();
        collect_paths(json, String::new(), 0, max_depth, &mut paths);
        for path in paths {
            self.collapsed.insert(path);
        }
        self
    }

    /// Collapse a specific path
    pub fn collapse_path(mut self, path: impl Into<String>) -> Self {
        self.collapsed.insert(path.into());
        self
    }

    /// Collapse multiple paths
    pub fn collapse_paths<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for path in paths {
            self.collapsed.insert(path.into());
        }
        self
    }

    /// Expand a specific path (remove from collapsed set)
    pub fn expand_path(mut self, path: impl Into<String>) -> Self {
        self.collapsed.remove(&path.into());
        self
    }

    /// Expand multiple paths
    pub fn expand_paths<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for path in paths {
            self.collapsed.remove(&path.into());
        }
        self
    }

    /// Build the final JsonTreeState
    pub fn build(self) -> JsonTreeState {
        JsonTreeState {
            collapsed: self.collapsed,
        }
    }
}

/// Try to parse text as JSON
pub fn try_parse_json(text: &str) -> Option<Value> {
    parse_json(text).ok()
}

// ==================== JSON Parse Error ====================

/// Error type for JSON parsing failures
///
/// Provides detailed information about why JSON parsing failed.
#[derive(Debug, Clone)]
pub enum JsonParseError {
    /// Input text is too short to be valid JSON (minimum is "{}" or "[]")
    TooShort,
    /// Input doesn't start with a valid JSON character ('{' or '[')
    InvalidStartChar(char),
    /// JSON parsing failed with serde_json error
    ParseError(String),
    /// Input is empty
    Empty,
}

impl std::fmt::Display for JsonParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonParseError::TooShort => write!(f, "input too short to be valid JSON"),
            JsonParseError::InvalidStartChar(c) => {
                write!(f, "invalid start character '{}', expected '{{' or '['", c)
            }
            JsonParseError::ParseError(e) => write!(f, "JSON parse error: {}", e),
            JsonParseError::Empty => write!(f, "empty input"),
        }
    }
}

impl std::error::Error for JsonParseError {}

impl From<serde_json::Error> for JsonParseError {
    fn from(err: serde_json::Error) -> Self {
        JsonParseError::ParseError(err.to_string())
    }
}

/// Parse text as JSON with detailed error information
///
/// Unlike `try_parse_json` which returns `Option`, this function
/// provides detailed error information about why parsing failed.
///
/// # Examples
/// ```
/// use lib_json_tree::{parse_json, JsonParseError};
///
/// // Successful parse
/// let result = parse_json(r#"{"name": "Alice"}"#);
/// assert!(result.is_ok());
///
/// // Too short
/// let result = parse_json("{");
/// assert!(matches!(result, Err(JsonParseError::TooShort)));
///
/// // Invalid start character
/// let result = parse_json("hello");
/// assert!(matches!(result, Err(JsonParseError::InvalidStartChar('h'))));
///
/// // Parse error
/// let result = parse_json(r#"{"name": }"#);
/// assert!(matches!(result, Err(JsonParseError::ParseError(_))));
/// ```
pub fn parse_json(text: &str) -> Result<Value, JsonParseError> {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return Err(JsonParseError::Empty);
    }

    if trimmed.len() < 2 {
        return Err(JsonParseError::TooShort);
    }

    let first_char = trimmed.chars().next().unwrap();
    if first_char != '{' && first_char != '[' {
        return Err(JsonParseError::InvalidStartChar(first_char));
    }

    serde_json::from_str(trimmed).map_err(JsonParseError::from)
}

/// Count the number of nodes in a JSON value
///
/// Works with any type implementing [`JsonValue`].
///
/// # Examples
/// ```
/// use lib_json_tree::count_nodes;
/// use serde_json::json;
///
/// let json = json!({"a": 1, "b": {"c": 2}});
/// assert_eq!(count_nodes(&json), 4); // root + a + b + c
/// ```
pub fn count_nodes<V: JsonValue>(value: &V) -> usize {
    if let Some(obj) = value.as_object() {
        1 + obj.values().map(count_nodes).sum::<usize>()
    } else if let Some(arr) = value.as_array() {
        1 + arr.iter().map(count_nodes).sum::<usize>()
    } else {
        1
    }
}

/// Format a JSON value as a compact preview string
///
/// Works with any type implementing [`JsonValue`].
///
/// # Examples
/// ```
/// use lib_json_tree::format_preview;
/// use serde_json::json;
///
/// let obj = json!({"a": 1, "b": 2});
/// assert_eq!(format_preview(&obj, 50), "{...} (2 keys)");
///
/// let s = json!("hello world");
/// assert_eq!(format_preview(&s, 5), "\"hello...\"");
/// ```
pub fn format_preview<V: JsonValue>(value: &V, max_len: usize) -> String {
    if let Some(obj) = value.as_object() {
        let count = obj.len();
        format!("{{...}} ({} keys)", count)
    } else if let Some(arr) = value.as_array() {
        let count = arr.len();
        format!("[...] ({} items)", count)
    } else if let Some(s) = value.as_str() {
        if s.len() > max_len {
            format!("\"{}...\"", &s[..max_len])
        } else {
            format!("\"{}\"", s)
        }
    } else if let Some(display) = value.to_display_string() {
        display
    } else {
        "null".to_string()
    }
}

/// JSON value type for coloring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonValueType {
    Object,
    Array,
    String,
    Number,
    Bool,
    Null,
    Key,
}

impl JsonValueType {
    /// Get the value type from any JSON value implementing [`JsonValue`]
    pub fn from_value<V: JsonValue>(value: &V) -> Self {
        value.value_type()
    }
}

/// A flattened JSON tree node for rendering
///
/// Uses `Cow<'a, str>` for string fields to avoid unnecessary allocations:
/// - `path`: Usually owned (built during traversal), but root path "" is borrowed
/// - `key`: Borrowed from JSON for object keys, owned for array indices
/// - `value_str`: Borrowed for static values like "null", "true", "false"
///
/// # Lifetime
/// The lifetime `'a` is tied to the JSON value being flattened. For owned nodes
/// (e.g., from iterator), use `JsonTreeNode<'static>` with `Cow::Owned` strings.
#[derive(Debug, Clone)]
pub struct JsonTreeNode<'a> {
    /// Path to this node (used as ID)
    pub path: Cow<'a, str>,
    /// Indentation level
    pub depth: usize,
    /// Key name (if object property)
    pub key: Option<Cow<'a, str>>,
    /// Value type
    pub value_type: JsonValueType,
    /// Value as string (for primitives)
    pub value_str: Option<Cow<'a, str>>,
    /// Number of children (for objects/arrays)
    pub child_count: usize,
    /// Is this node collapsible (has children)
    pub collapsible: bool,
    /// Is this node currently collapsed
    pub is_collapsed: bool,
}

impl<'a> JsonTreeNode<'a> {
    /// Convert to an owned version with 'static lifetime
    ///
    /// This is useful when you need to store nodes beyond the lifetime
    /// of the JSON value, or when collecting from an iterator.
    pub fn into_owned(self) -> JsonTreeNode<'static> {
        JsonTreeNode {
            path: Cow::Owned(self.path.into_owned()),
            depth: self.depth,
            key: self.key.map(|k| Cow::Owned(k.into_owned())),
            value_type: self.value_type,
            value_str: self.value_str.map(|v| Cow::Owned(v.into_owned())),
            child_count: self.child_count,
            collapsible: self.collapsible,
            is_collapsed: self.is_collapsed,
        }
    }

    /// Get the path as a string slice
    pub fn path_str(&self) -> &str {
        &self.path
    }

    /// Get the key as an optional string slice
    pub fn key_str(&self) -> Option<&str> {
        self.key.as_deref()
    }

    /// Get the value string as an optional string slice
    pub fn value_str_ref(&self) -> Option<&str> {
        self.value_str.as_deref()
    }
}

/// Flatten a JSON value into a list of tree nodes for rendering
///
/// Works with any type implementing [`JsonValue`].
///
/// The returned nodes have a lifetime tied to the input JSON value,
/// allowing them to borrow strings (like object keys) directly from the JSON.
///
/// # Examples
/// ```
/// use lib_json_tree::{flatten_json, JsonTreeState};
/// use serde_json::json;
///
/// let json = json!({"name": "Alice", "age": 30});
/// let state = JsonTreeState::new();
/// let nodes = flatten_json(&json, &state);
/// assert_eq!(nodes.len(), 3); // root object + 2 fields
/// ```
pub fn flatten_json<'a, V: JsonValue>(
    value: &'a V,
    state: &JsonTreeState,
) -> Vec<JsonTreeNode<'a>> {
    // Pre-allocate based on node count for better performance
    let mut nodes = Vec::with_capacity(count_nodes(value));
    flatten_recursive(value, Cow::Borrowed(""), 0, None, state, &mut nodes);
    nodes
}

/// Flatten a JSON value into a SmallVec for small trees
///
/// This function is optimized for small JSON structures (≤32 nodes) where
/// it avoids heap allocation entirely by using stack-based storage.
///
/// Works with any type implementing [`JsonValue`].
///
/// The returned nodes have a lifetime tied to the input JSON value.
///
/// # Examples
/// ```
/// use lib_json_tree::{flatten_json_small, JsonTreeState};
/// use serde_json::json;
///
/// let json = json!({"name": "Alice", "age": 30});
/// let state = JsonTreeState::new();
/// let nodes = flatten_json_small(&json, &state);
/// // For small JSON, this avoids heap allocation
/// assert_eq!(nodes.len(), 3); // root + 2 fields
/// ```
pub fn flatten_json_small<'a, V: JsonValue>(
    value: &'a V,
    state: &JsonTreeState,
) -> SmallNodeVec<'a> {
    let mut nodes = SmallNodeVec::new();
    flatten_recursive_smallvec(value, Cow::Borrowed(""), 0, None, state, &mut nodes);
    nodes
}

fn flatten_recursive_smallvec<'a, V: JsonValue>(
    value: &'a V,
    path: Cow<'a, str>,
    depth: usize,
    key: Option<Cow<'a, str>>,
    state: &JsonTreeState,
    nodes: &mut SmallNodeVec<'a>,
) {
    let value_type = value.value_type();
    let is_collapsed = state.is_collapsed(&path);

    if let Some(obj) = value.as_object() {
        let child_count = obj.len();
        nodes.push(JsonTreeNode {
            path: path.clone(),
            depth,
            key,
            value_type,
            value_str: None,
            child_count,
            collapsible: child_count > 0,
            is_collapsed,
        });

        if !is_collapsed {
            for (k, v) in obj.iter() {
                let child_path = build_child_path(&path, PathSegment::Key(k));
                flatten_recursive_smallvec(
                    v,
                    Cow::Owned(child_path),
                    depth + 1,
                    Some(Cow::Borrowed(k)),
                    state,
                    nodes,
                );
            }
        }
    } else if let Some(arr) = value.as_array() {
        let child_count = arr.len();
        nodes.push(JsonTreeNode {
            path: path.clone(),
            depth,
            key,
            value_type,
            value_str: None,
            child_count,
            collapsible: child_count > 0,
            is_collapsed,
        });

        if !is_collapsed {
            for (idx, v) in arr.iter().enumerate() {
                let child_path = build_child_path(&path, PathSegment::Index(idx));
                flatten_recursive_smallvec(
                    v,
                    Cow::Owned(child_path),
                    depth + 1,
                    Some(Cow::Owned(format!("[{}]", idx))),
                    state,
                    nodes,
                );
            }
        }
    } else {
        // Primitive value
        nodes.push(JsonTreeNode {
            path,
            depth,
            key,
            value_type,
            value_str: value_to_cow(value),
            child_count: 0,
            collapsible: false,
            is_collapsed: false,
        });
    }
}

/// Convert a JSON value to a Cow string for display
///
/// Uses static strings for common values (null, true, false) to avoid allocation.
fn value_to_cow<'a, V: JsonValue>(value: &V) -> Option<Cow<'a, str>> {
    if value.is_null() {
        Some(Cow::Borrowed("null"))
    } else if let Some(b) = value.as_bool() {
        Some(Cow::Borrowed(if b { "true" } else { "false" }))
    } else {
        value.to_display_string().map(Cow::Owned)
    }
}

fn flatten_recursive<'a, V: JsonValue>(
    value: &'a V,
    path: Cow<'a, str>,
    depth: usize,
    key: Option<Cow<'a, str>>,
    state: &JsonTreeState,
    nodes: &mut Vec<JsonTreeNode<'a>>,
) {
    let value_type = value.value_type();
    let is_collapsed = state.is_collapsed(&path);

    if let Some(obj) = value.as_object() {
        let child_count = obj.len();
        nodes.push(JsonTreeNode {
            path: path.clone(),
            depth,
            key,
            value_type,
            value_str: None,
            child_count,
            collapsible: child_count > 0,
            is_collapsed,
        });

        if !is_collapsed {
            for (k, v) in obj.iter() {
                let child_path = build_child_path(&path, PathSegment::Key(k));
                flatten_recursive(
                    v,
                    Cow::Owned(child_path),
                    depth + 1,
                    Some(Cow::Borrowed(k)),
                    state,
                    nodes,
                );
            }
        }
    } else if let Some(arr) = value.as_array() {
        let child_count = arr.len();
        nodes.push(JsonTreeNode {
            path: path.clone(),
            depth,
            key,
            value_type,
            value_str: None,
            child_count,
            collapsible: child_count > 0,
            is_collapsed,
        });

        if !is_collapsed {
            for (idx, v) in arr.iter().enumerate() {
                let child_path = build_child_path(&path, PathSegment::Index(idx));
                flatten_recursive(
                    v,
                    Cow::Owned(child_path),
                    depth + 1,
                    Some(Cow::Owned(format!("[{}]", idx))),
                    state,
                    nodes,
                );
            }
        }
    } else {
        // Primitive value
        nodes.push(JsonTreeNode {
            path,
            depth,
            key,
            value_type,
            value_str: value_to_cow(value),
            child_count: 0,
            collapsible: false,
            is_collapsed: false,
        });
    }
}

// ==================== Iterator Support ====================

/// Iterator state for traversing JSON tree
struct IteratorFrame<'a> {
    /// The JSON value at this frame
    value: &'a Value,
    /// Path to this value (owned for simplicity in iteration)
    path: String,
    /// Depth of this value
    depth: usize,
    /// Key name (if object property, owned for simplicity)
    key: Option<String>,
    /// Index into children (for objects/arrays)
    child_index: usize,
    /// Whether we've yielded the node itself
    yielded_self: bool,
}

/// Lazy iterator over JSON tree nodes
///
/// This iterator traverses the JSON tree on-demand, yielding nodes as they're requested.
/// Unlike `flatten_json`, it doesn't pre-allocate or compute all nodes upfront.
///
/// **Note**: The iterator yields `JsonTreeNode<'static>` with owned strings for simplicity,
/// as storing borrowed references during iteration would require self-referential structs.
/// For borrowing optimizations, use `flatten_json` or `flatten_json_small` instead.
///
/// # Examples
/// ```
/// use lib_json_tree::{flatten_json_iter, JsonTreeState};
/// use serde_json::json;
///
/// let json = json!({"users": [{"name": "Alice"}]});
/// let state = JsonTreeState::new();
///
/// // Iterate lazily
/// for node in flatten_json_iter(&json, &state) {
///     println!("{}: {:?}", node.path, node.value_type);
/// }
///
/// // Collect only first 2 nodes
/// let first_two: Vec<_> = flatten_json_iter(&json, &state).take(2).collect();
/// assert_eq!(first_two.len(), 2);
/// ```
pub struct JsonTreeIterator<'a> {
    state: &'a JsonTreeState,
    stack: Vec<IteratorFrame<'a>>,
}

impl<'a> JsonTreeIterator<'a> {
    fn new(value: &'a Value, state: &'a JsonTreeState) -> Self {
        Self {
            state,
            stack: vec![IteratorFrame {
                value,
                path: String::new(),
                depth: 0,
                key: None,
                child_index: 0,
                yielded_self: false,
            }],
        }
    }

    fn create_node(&self, frame: &IteratorFrame<'a>) -> JsonTreeNode<'static> {
        let value_type = JsonValueType::from_value(frame.value);
        let is_collapsed = self.state.is_collapsed(&frame.path);

        match frame.value {
            Value::Object(map) => {
                let child_count = map.len();
                JsonTreeNode {
                    path: Cow::Owned(frame.path.clone()),
                    depth: frame.depth,
                    key: frame.key.clone().map(Cow::Owned),
                    value_type,
                    value_str: None,
                    child_count,
                    collapsible: child_count > 0,
                    is_collapsed,
                }
            }
            Value::Array(arr) => {
                let child_count = arr.len();
                JsonTreeNode {
                    path: Cow::Owned(frame.path.clone()),
                    depth: frame.depth,
                    key: frame.key.clone().map(Cow::Owned),
                    value_type,
                    value_str: None,
                    child_count,
                    collapsible: child_count > 0,
                    is_collapsed,
                }
            }
            Value::String(s) => JsonTreeNode {
                path: Cow::Owned(frame.path.clone()),
                depth: frame.depth,
                key: frame.key.clone().map(Cow::Owned),
                value_type,
                value_str: Some(Cow::Owned(format!("\"{}\"", s))),
                child_count: 0,
                collapsible: false,
                is_collapsed: false,
            },
            Value::Number(n) => JsonTreeNode {
                path: Cow::Owned(frame.path.clone()),
                depth: frame.depth,
                key: frame.key.clone().map(Cow::Owned),
                value_type,
                value_str: Some(Cow::Owned(n.to_string())),
                child_count: 0,
                collapsible: false,
                is_collapsed: false,
            },
            Value::Bool(b) => JsonTreeNode {
                path: Cow::Owned(frame.path.clone()),
                depth: frame.depth,
                key: frame.key.clone().map(Cow::Owned),
                value_type,
                value_str: Some(Cow::Borrowed(if *b { "true" } else { "false" })),
                child_count: 0,
                collapsible: false,
                is_collapsed: false,
            },
            Value::Null => JsonTreeNode {
                path: Cow::Owned(frame.path.clone()),
                depth: frame.depth,
                key: frame.key.clone().map(Cow::Owned),
                value_type,
                value_str: Some(Cow::Borrowed("null")),
                child_count: 0,
                collapsible: false,
                is_collapsed: false,
            },
        }
    }
}

impl<'a> Iterator for JsonTreeIterator<'a> {
    type Item = JsonTreeNode<'static>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let stack_len = self.stack.len();
            if stack_len == 0 {
                return None;
            }

            let frame = &self.stack[stack_len - 1];

            // If we haven't yielded this node yet, do so now
            if !frame.yielded_self {
                let node = self.create_node(frame);
                self.stack[stack_len - 1].yielded_self = true;
                return Some(node);
            }

            // Check if this node is collapsed or is a primitive
            let is_collapsed = self.state.is_collapsed(&frame.path);

            // Extract what we need before modifying the stack
            let child_index = frame.child_index;
            let depth = frame.depth;
            let path = frame.path.clone();

            match frame.value {
                Value::Object(map) if !is_collapsed => {
                    let keys: Vec<_> = map.keys().collect();
                    if child_index < keys.len() {
                        let key = keys[child_index];
                        let child_value = &map[key];
                        let child_path = build_child_path(&path, PathSegment::Key(key));
                        let key_clone = key.clone();

                        self.stack[stack_len - 1].child_index += 1;
                        self.stack.push(IteratorFrame {
                            value: child_value,
                            path: child_path,
                            depth: depth + 1,
                            key: Some(key_clone),
                            child_index: 0,
                            yielded_self: false,
                        });
                        continue;
                    }
                }
                Value::Array(arr) if !is_collapsed => {
                    if child_index < arr.len() {
                        let child_value = &arr[child_index];
                        let child_path = build_child_path(&path, PathSegment::Index(child_index));

                        self.stack[stack_len - 1].child_index += 1;
                        self.stack.push(IteratorFrame {
                            value: child_value,
                            path: child_path,
                            depth: depth + 1,
                            key: Some(format!("[{}]", child_index)),
                            child_index: 0,
                            yielded_self: false,
                        });
                        continue;
                    }
                }
                _ => {}
            }

            // Done with this frame, pop it
            self.stack.pop();
        }
    }
}

/// Return an iterator over JSON tree nodes for lazy evaluation
///
/// Unlike `flatten_json`, this doesn't pre-compute all nodes. Useful for:
/// - Large JSON where you only need a few nodes
/// - Early termination scenarios
/// - Memory-constrained environments
///
/// # Examples
/// ```
/// use lib_json_tree::{flatten_json_iter, JsonTreeState};
/// use serde_json::json;
///
/// let json = json!({"a": 1, "b": {"c": 2}});
/// let state = JsonTreeState::new();
///
/// // Lazy iteration
/// let count = flatten_json_iter(&json, &state).count();
/// assert_eq!(count, 4);
///
/// // Find first matching node
/// let node = flatten_json_iter(&json, &state)
///     .find(|n| n.path == "b.c");
/// assert!(node.is_some());
/// ```
pub fn flatten_json_iter<'a>(value: &'a Value, state: &'a JsonTreeState) -> JsonTreeIterator<'a> {
    JsonTreeIterator::new(value, state)
}

// ==================== JSON Pointer (RFC 6901) Support ====================

/// Get a value from JSON using a JSON Pointer (RFC 6901)
///
/// # Examples
/// ```
/// use lib_json_tree::get_by_pointer;
/// use serde_json::json;
///
/// let json = json!({"users": [{"name": "Alice"}]});
/// assert_eq!(get_by_pointer(&json, "/users/0/name"), Some(&json!("Alice")));
/// assert_eq!(get_by_pointer(&json, ""), Some(&json));
/// assert_eq!(get_by_pointer(&json, "/nonexistent"), None);
/// ```
pub fn get_by_pointer<'a>(value: &'a Value, pointer: &str) -> Option<&'a Value> {
    if pointer.is_empty() {
        return Some(value);
    }

    if !pointer.starts_with('/') {
        return None;
    }

    let mut current = value;
    for token in pointer[1..].split('/') {
        // Unescape JSON Pointer escape sequences: ~1 -> /, ~0 -> ~
        let unescaped = token.replace("~1", "/").replace("~0", "~");

        current = match current {
            Value::Object(map) => map.get(&unescaped)?,
            Value::Array(arr) => {
                let idx: usize = unescaped.parse().ok()?;
                arr.get(idx)?
            }
            _ => return None,
        };
    }
    Some(current)
}

/// Convert a dot-notation path to a JSON Pointer (RFC 6901)
///
/// # Examples
/// ```
/// use lib_json_tree::path_to_pointer;
///
/// assert_eq!(path_to_pointer(""), "");
/// assert_eq!(path_to_pointer("users"), "/users");
/// assert_eq!(path_to_pointer("users[0].name"), "/users/0/name");
/// assert_eq!(path_to_pointer("[0][1]"), "/0/1");
/// ```
pub fn path_to_pointer(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut chars = path.chars().peekable();
    let mut current_segment = String::new();

    while let Some(ch) = chars.next() {
        match ch {
            '.' => {
                if !current_segment.is_empty() {
                    result.push('/');
                    // Escape ~ and / per RFC 6901
                    result.push_str(&escape_pointer_token(&current_segment));
                    current_segment.clear();
                }
            }
            '[' => {
                if !current_segment.is_empty() {
                    result.push('/');
                    result.push_str(&escape_pointer_token(&current_segment));
                    current_segment.clear();
                }
                // Read the index
                while let Some(&c) = chars.peek() {
                    if c == ']' {
                        chars.next();
                        break;
                    }
                    current_segment.push(chars.next().unwrap());
                }
                result.push('/');
                result.push_str(&current_segment);
                current_segment.clear();
            }
            ']' => {
                // Should have been consumed above, but handle gracefully
            }
            _ => {
                current_segment.push(ch);
            }
        }
    }

    if !current_segment.is_empty() {
        result.push('/');
        result.push_str(&escape_pointer_token(&current_segment));
    }

    result
}

/// Convert a JSON Pointer (RFC 6901) to dot-notation path
///
/// # Examples
/// ```
/// use lib_json_tree::pointer_to_path;
///
/// assert_eq!(pointer_to_path(""), "");
/// assert_eq!(pointer_to_path("/users"), "users");
/// assert_eq!(pointer_to_path("/users/0/name"), "users[0].name");
/// assert_eq!(pointer_to_path("/0/1"), "[0][1]");
/// ```
pub fn pointer_to_path(pointer: &str) -> String {
    if pointer.is_empty() {
        return String::new();
    }

    if !pointer.starts_with('/') {
        return pointer.to_string(); // Invalid pointer, return as-is
    }

    let mut result = String::new();
    let mut first = true;

    for token in pointer[1..].split('/') {
        // Unescape JSON Pointer escape sequences
        let unescaped = token.replace("~1", "/").replace("~0", "~");

        // Check if it's a numeric index
        if unescaped.chars().all(|c| c.is_ascii_digit()) {
            result.push('[');
            result.push_str(&unescaped);
            result.push(']');
        } else {
            if !first {
                result.push('.');
            }
            result.push_str(&unescaped);
        }
        first = false;
    }

    result
}

/// Escape a token for use in a JSON Pointer (RFC 6901)
fn escape_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

// ==================== Path Format Conversions ====================

impl JsonTreeNode<'_> {
    /// Convert path to JSON Pointer (RFC 6901) format
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::{flatten_json, JsonTreeState};
    /// use serde_json::json;
    ///
    /// let json = json!({"users": [{"name": "Alice"}]});
    /// let state = JsonTreeState::new();
    /// let nodes = flatten_json(&json, &state);
    /// let name_node = nodes.iter().find(|n| n.path == "users[0].name").unwrap();
    /// assert_eq!(name_node.to_json_pointer(), "/users/0/name");
    /// ```
    pub fn to_json_pointer(&self) -> String {
        path_to_pointer(&self.path)
    }

    /// Convert path to jq path format (e.g., .users[0].name)
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::{flatten_json, JsonTreeState};
    /// use serde_json::json;
    ///
    /// let json = json!({"users": [{"name": "Alice"}]});
    /// let state = JsonTreeState::new();
    /// let nodes = flatten_json(&json, &state);
    /// let name_node = nodes.iter().find(|n| n.path == "users[0].name").unwrap();
    /// assert_eq!(name_node.to_jq_path(), ".users[0].name");
    /// ```
    pub fn to_jq_path(&self) -> String {
        if self.path.is_empty() {
            return ".".to_string();
        }

        let mut result = String::from(".");
        let mut chars = self.path.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '[' => {
                    result.push('[');
                    // Copy index and closing bracket
                    while let Some(&c) = chars.peek() {
                        result.push(chars.next().unwrap());
                        if c == ']' {
                            break;
                        }
                    }
                }
                _ => result.push(ch),
            }
        }

        result
    }

    /// Convert path to bracket notation (e.g., ["users"][0]["name"])
    ///
    /// # Examples
    /// ```
    /// use lib_json_tree::{flatten_json, JsonTreeState};
    /// use serde_json::json;
    ///
    /// let json = json!({"users": [{"name": "Alice"}]});
    /// let state = JsonTreeState::new();
    /// let nodes = flatten_json(&json, &state);
    /// let name_node = nodes.iter().find(|n| n.path == "users[0].name").unwrap();
    /// assert_eq!(name_node.to_bracket_notation(), r#"["users"][0]["name"]"#);
    /// ```
    pub fn to_bracket_notation(&self) -> String {
        if self.path.is_empty() {
            return String::new();
        }

        let mut result = String::new();
        let mut chars = self.path.chars().peekable();
        let mut current_key = String::new();

        while let Some(ch) = chars.next() {
            match ch {
                '.' => {
                    if !current_key.is_empty() {
                        result.push_str(&format!("[\"{}\"]", current_key));
                        current_key.clear();
                    }
                }
                '[' => {
                    if !current_key.is_empty() {
                        result.push_str(&format!("[\"{}\"]", current_key));
                        current_key.clear();
                    }
                    // Read and output the index
                    result.push('[');
                    while let Some(&c) = chars.peek() {
                        if c == ']' {
                            result.push(chars.next().unwrap());
                            break;
                        }
                        result.push(chars.next().unwrap());
                    }
                }
                _ => {
                    current_key.push(ch);
                }
            }
        }

        if !current_key.is_empty() {
            result.push_str(&format!("[\"{}\"]", current_key));
        }

        result
    }
}

// ==================== Depth-Limited Flattening ====================

/// Flatten a JSON value into a list of tree nodes, limited to a maximum depth
///
/// Nodes beyond max_depth are not traversed, improving performance for large JSON.
/// Unlike `flatten_json`, this doesn't require all nodes to be tracked in state.
///
/// The returned nodes have a lifetime tied to the input JSON value.
///
/// # Examples
/// ```
/// use lib_json_tree::{flatten_json_to_depth, JsonTreeState};
/// use serde_json::json;
///
/// let json = json!({"a": {"b": {"c": {"d": 1}}}});
/// let state = JsonTreeState::new();
///
/// // Flatten only to depth 2
/// let nodes = flatten_json_to_depth(&json, &state, 2);
/// // Should have: root (depth 0), a (depth 1), b (depth 2)
/// // c is at depth 3, so it's not included
/// assert_eq!(nodes.len(), 3);
/// ```
pub fn flatten_json_to_depth<'a, V: JsonValue>(
    value: &'a V,
    state: &JsonTreeState,
    max_depth: usize,
) -> Vec<JsonTreeNode<'a>> {
    let mut nodes = Vec::with_capacity(count_nodes(value).min(1000)); // Cap pre-allocation for large JSON
    flatten_recursive_depth_limited(
        value,
        Cow::Borrowed(""),
        0,
        None,
        state,
        max_depth,
        &mut nodes,
    );
    nodes
}

fn flatten_recursive_depth_limited<'a, V: JsonValue>(
    value: &'a V,
    path: Cow<'a, str>,
    depth: usize,
    key: Option<Cow<'a, str>>,
    state: &JsonTreeState,
    max_depth: usize,
    nodes: &mut Vec<JsonTreeNode<'a>>,
) {
    let value_type = value.value_type();
    let is_collapsed = state.is_collapsed(&path);
    let at_depth_limit = depth >= max_depth;

    if let Some(obj) = value.as_object() {
        let child_count = obj.len();
        nodes.push(JsonTreeNode {
            path: path.clone(),
            depth,
            key,
            value_type,
            value_str: None,
            child_count,
            collapsible: child_count > 0,
            is_collapsed: is_collapsed || at_depth_limit,
        });

        if !is_collapsed && !at_depth_limit {
            for (k, v) in obj.iter() {
                let child_path = build_child_path(&path, PathSegment::Key(k));
                flatten_recursive_depth_limited(
                    v,
                    Cow::Owned(child_path),
                    depth + 1,
                    Some(Cow::Borrowed(k)),
                    state,
                    max_depth,
                    nodes,
                );
            }
        }
    } else if let Some(arr) = value.as_array() {
        let child_count = arr.len();
        nodes.push(JsonTreeNode {
            path: path.clone(),
            depth,
            key,
            value_type,
            value_str: None,
            child_count,
            collapsible: child_count > 0,
            is_collapsed: is_collapsed || at_depth_limit,
        });

        if !is_collapsed && !at_depth_limit {
            for (idx, v) in arr.iter().enumerate() {
                let child_path = build_child_path(&path, PathSegment::Index(idx));
                flatten_recursive_depth_limited(
                    v,
                    Cow::Owned(child_path),
                    depth + 1,
                    Some(Cow::Owned(format!("[{}]", idx))),
                    state,
                    max_depth,
                    nodes,
                );
            }
        }
    } else {
        // Primitive value
        nodes.push(JsonTreeNode {
            path,
            depth,
            key,
            value_type,
            value_str: value_to_cow(value),
            child_count: 0,
            collapsible: false,
            is_collapsed: false,
        });
    }
}

// ==================== Search/Filter Functionality ====================

/// Filter tree nodes by a case-insensitive query string
///
/// Matches against key names, value strings, and paths.
///
/// # Examples
/// ```
/// use lib_json_tree::{flatten_json, filter_nodes, JsonTreeState};
/// use serde_json::json;
///
/// let json = json!({"name": "Alice", "age": 30});
/// let state = JsonTreeState::new();
/// let nodes = flatten_json(&json, &state);
/// let filtered = filter_nodes(&nodes, "alice");
/// assert_eq!(filtered.len(), 1);
/// ```
pub fn filter_nodes<'a, 'b>(
    nodes: &'a [JsonTreeNode<'b>],
    query: &str,
) -> Vec<&'a JsonTreeNode<'b>> {
    let query_lower = query.to_lowercase();
    nodes
        .iter()
        .filter(|node| {
            // Match against key
            if let Some(key) = &node.key {
                if key.to_lowercase().contains(&query_lower) {
                    return true;
                }
            }
            // Match against value string
            if let Some(val) = &node.value_str {
                if val.to_lowercase().contains(&query_lower) {
                    return true;
                }
            }
            // Match against path
            if node.path.to_lowercase().contains(&query_lower) {
                return true;
            }
            false
        })
        .collect()
}

/// Find the path to a value matching a predicate
///
/// Returns the first path where the predicate returns true.
/// Works with any type implementing [`JsonValue`].
///
/// # Examples
/// ```
/// use lib_json_tree::find_path;
/// use serde_json::json;
///
/// let json = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});
/// let path = find_path(&json, |v| v.as_str() == Some("Bob"));
/// assert_eq!(path, Some("users[1].name".to_string()));
/// ```
pub fn find_path<V: JsonValue>(value: &V, predicate: impl Fn(&V) -> bool) -> Option<String> {
    find_path_recursive(value, String::new(), &predicate)
}

fn find_path_recursive<V: JsonValue>(
    value: &V,
    path: String,
    predicate: &impl Fn(&V) -> bool,
) -> Option<String> {
    if predicate(value) {
        return Some(path);
    }

    if let Some(obj) = value.as_object() {
        for (key, val) in obj.iter() {
            let child_path = build_child_path(&path, PathSegment::Key(key));
            if let Some(found) = find_path_recursive(val, child_path, predicate) {
                return Some(found);
            }
        }
    } else if let Some(arr) = value.as_array() {
        for (idx, val) in arr.iter().enumerate() {
            let child_path = build_child_path(&path, PathSegment::Index(idx));
            if let Some(found) = find_path_recursive(val, child_path, predicate) {
                return Some(found);
            }
        }
    }
    None
}

/// Find all paths to values matching a predicate
///
/// Works with any type implementing [`JsonValue`].
///
/// # Examples
/// ```
/// use lib_json_tree::find_all_paths;
/// use serde_json::json;
///
/// let json = json!({"a": 1, "b": {"c": 1}});
/// let paths = find_all_paths(&json, |v| v.as_i64() == Some(1));
/// assert_eq!(paths.len(), 2);
/// ```
pub fn find_all_paths<V: JsonValue>(value: &V, predicate: impl Fn(&V) -> bool) -> Vec<String> {
    let mut paths = Vec::new();
    find_all_paths_recursive(value, String::new(), &predicate, &mut paths);
    paths
}

fn find_all_paths_recursive<V: JsonValue>(
    value: &V,
    path: String,
    predicate: &impl Fn(&V) -> bool,
    paths: &mut Vec<String>,
) {
    if predicate(value) {
        paths.push(path.clone());
    }

    if let Some(obj) = value.as_object() {
        for (key, val) in obj.iter() {
            let child_path = build_child_path(&path, PathSegment::Key(key));
            find_all_paths_recursive(val, child_path, predicate, paths);
        }
    } else if let Some(arr) = value.as_array() {
        for (idx, val) in arr.iter().enumerate() {
            let child_path = build_child_path(&path, PathSegment::Index(idx));
            find_all_paths_recursive(val, child_path, predicate, paths);
        }
    }
}

// ==================== JSON Diff Support ====================

/// Type of change in a JSON diff
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffType {
    /// Value was added (exists only in right)
    Added,
    /// Value was removed (exists only in left)
    Removed,
    /// Value was modified (different values)
    Modified,
    /// Value is unchanged
    Unchanged,
}

/// A node in the JSON diff tree
#[derive(Debug, Clone)]
pub struct DiffNode {
    /// Path to this node
    pub path: String,
    /// Key name (if object property)
    pub key: Option<String>,
    /// Indentation level
    pub depth: usize,
    /// Type of change at this node
    pub diff_type: DiffType,
    /// Left (old) value type
    pub left_type: Option<JsonValueType>,
    /// Right (new) value type
    pub right_type: Option<JsonValueType>,
    /// Left (old) value as string (for primitives)
    pub left_value: Option<String>,
    /// Right (new) value as string (for primitives)
    pub right_value: Option<String>,
    /// Number of children on left side
    pub left_child_count: usize,
    /// Number of children on right side
    pub right_child_count: usize,
    /// Is this node collapsible
    pub collapsible: bool,
}

/// Compare two JSON values and produce a flat list of diff nodes for rendering
///
/// # Examples
/// ```
/// use lib_json_tree::{diff_json, DiffType};
/// use serde_json::json;
///
/// let old = json!({"name": "Alice", "age": 30});
/// let new = json!({"name": "Alice", "age": 31, "city": "NYC"});
///
/// let diffs = diff_json(&old, &new);
///
/// // Find the modified age
/// let age_diff = diffs.iter().find(|d| d.path == "age").unwrap();
/// assert_eq!(age_diff.diff_type, DiffType::Modified);
/// assert_eq!(age_diff.left_value, Some("30".to_string()));
/// assert_eq!(age_diff.right_value, Some("31".to_string()));
///
/// // Find the added city
/// let city_diff = diffs.iter().find(|d| d.path == "city").unwrap();
/// assert_eq!(city_diff.diff_type, DiffType::Added);
/// ```
/// Compare two JSON values and produce a diff tree
///
/// Works with any type implementing [`JsonValue`].
pub fn diff_json<V: JsonValue>(left: &V, right: &V) -> Vec<DiffNode> {
    let mut nodes = Vec::new();
    diff_recursive(left, right, String::new(), 0, None, &mut nodes);
    nodes
}

fn diff_recursive<V: JsonValue>(
    left: &V,
    right: &V,
    path: String,
    depth: usize,
    key: Option<String>,
    nodes: &mut Vec<DiffNode>,
) {
    // If values are equal, mark as unchanged
    if left == right {
        add_unchanged_node(left, path, depth, key, nodes);
        return;
    }

    // If types differ, it's a modification
    if !left.same_type(right) {
        nodes.push(DiffNode {
            path,
            key,
            depth,
            diff_type: DiffType::Modified,
            left_type: Some(left.value_type()),
            right_type: Some(right.value_type()),
            left_value: value_to_string(left),
            right_value: value_to_string(right),
            left_child_count: count_children(left),
            right_child_count: count_children(right),
            collapsible: false,
        });
        return;
    }

    // Same type but different content
    if let (Some(left_obj), Some(right_obj)) = (left.as_object(), right.as_object()) {
        // Collect all keys from both objects
        let left_keys: std::collections::HashSet<_> = left_obj.keys().collect();
        let right_keys: std::collections::HashSet<_> = right_obj.keys().collect();
        let all_keys: std::collections::HashSet<_> =
            left_keys.union(&right_keys).copied().collect();

        let left_count = left_obj.len();
        let right_count = right_obj.len();

        nodes.push(DiffNode {
            path: path.clone(),
            key,
            depth,
            diff_type: DiffType::Modified,
            left_type: Some(JsonValueType::Object),
            right_type: Some(JsonValueType::Object),
            left_value: None,
            right_value: None,
            left_child_count: left_count,
            right_child_count: right_count,
            collapsible: left_count > 0 || right_count > 0,
        });

        for k in all_keys {
            let child_path = build_child_path(&path, PathSegment::Key(k));
            match (left_obj.get(k), right_obj.get(k)) {
                (Some(lv), Some(rv)) => {
                    diff_recursive(lv, rv, child_path, depth + 1, Some(k.to_string()), nodes);
                }
                (Some(lv), None) => {
                    add_removed_node(lv, child_path, depth + 1, Some(k.to_string()), nodes);
                }
                (None, Some(rv)) => {
                    add_added_node(rv, child_path, depth + 1, Some(k.to_string()), nodes);
                }
                (None, None) => unreachable!(),
            }
        }
    } else if let (Some(left_arr), Some(right_arr)) = (left.as_array(), right.as_array()) {
        let left_count = left_arr.len();
        let right_count = right_arr.len();

        nodes.push(DiffNode {
            path: path.clone(),
            key,
            depth,
            diff_type: DiffType::Modified,
            left_type: Some(JsonValueType::Array),
            right_type: Some(JsonValueType::Array),
            left_value: None,
            right_value: None,
            left_child_count: left_count,
            right_child_count: right_count,
            collapsible: left_count > 0 || right_count > 0,
        });

        let max_len = left_count.max(right_count);
        for idx in 0..max_len {
            let child_path = build_child_path(&path, PathSegment::Index(idx));
            match (left_arr.get(idx), right_arr.get(idx)) {
                (Some(lv), Some(rv)) => {
                    diff_recursive(
                        lv,
                        rv,
                        child_path,
                        depth + 1,
                        Some(format!("[{}]", idx)),
                        nodes,
                    );
                }
                (Some(lv), None) => {
                    add_removed_node(lv, child_path, depth + 1, Some(format!("[{}]", idx)), nodes);
                }
                (None, Some(rv)) => {
                    add_added_node(rv, child_path, depth + 1, Some(format!("[{}]", idx)), nodes);
                }
                (None, None) => unreachable!(),
            }
        }
    } else {
        // Primitive types with different values
        nodes.push(DiffNode {
            path,
            key,
            depth,
            diff_type: DiffType::Modified,
            left_type: Some(left.value_type()),
            right_type: Some(right.value_type()),
            left_value: value_to_string(left),
            right_value: value_to_string(right),
            left_child_count: 0,
            right_child_count: 0,
            collapsible: false,
        });
    }
}

fn add_unchanged_node<V: JsonValue>(
    value: &V,
    path: String,
    depth: usize,
    key: Option<String>,
    nodes: &mut Vec<DiffNode>,
) {
    let value_type = value.value_type();
    let child_count = count_children(value);

    nodes.push(DiffNode {
        path: path.clone(),
        key,
        depth,
        diff_type: DiffType::Unchanged,
        left_type: Some(value_type),
        right_type: Some(value_type),
        left_value: value_to_string(value),
        right_value: value_to_string(value),
        left_child_count: child_count,
        right_child_count: child_count,
        collapsible: child_count > 0,
    });

    // Recurse into children for unchanged containers
    if let Some(obj) = value.as_object() {
        for (k, v) in obj.iter() {
            let child_path = build_child_path(&path, PathSegment::Key(k));
            add_unchanged_node(v, child_path, depth + 1, Some(k.to_string()), nodes);
        }
    } else if let Some(arr) = value.as_array() {
        for (idx, v) in arr.iter().enumerate() {
            let child_path = build_child_path(&path, PathSegment::Index(idx));
            add_unchanged_node(v, child_path, depth + 1, Some(format!("[{}]", idx)), nodes);
        }
    }
}

fn add_removed_node<V: JsonValue>(
    value: &V,
    path: String,
    depth: usize,
    key: Option<String>,
    nodes: &mut Vec<DiffNode>,
) {
    let value_type = value.value_type();
    let child_count = count_children(value);

    nodes.push(DiffNode {
        path: path.clone(),
        key,
        depth,
        diff_type: DiffType::Removed,
        left_type: Some(value_type),
        right_type: None,
        left_value: value_to_string(value),
        right_value: None,
        left_child_count: child_count,
        right_child_count: 0,
        collapsible: child_count > 0,
    });

    // Recurse into children for removed containers
    if let Some(obj) = value.as_object() {
        for (k, v) in obj.iter() {
            let child_path = build_child_path(&path, PathSegment::Key(k));
            add_removed_node(v, child_path, depth + 1, Some(k.to_string()), nodes);
        }
    } else if let Some(arr) = value.as_array() {
        for (idx, v) in arr.iter().enumerate() {
            let child_path = build_child_path(&path, PathSegment::Index(idx));
            add_removed_node(v, child_path, depth + 1, Some(format!("[{}]", idx)), nodes);
        }
    }
}

fn add_added_node<V: JsonValue>(
    value: &V,
    path: String,
    depth: usize,
    key: Option<String>,
    nodes: &mut Vec<DiffNode>,
) {
    let value_type = value.value_type();
    let child_count = count_children(value);

    nodes.push(DiffNode {
        path: path.clone(),
        key,
        depth,
        diff_type: DiffType::Added,
        left_type: None,
        right_type: Some(value_type),
        left_value: None,
        right_value: value_to_string(value),
        left_child_count: 0,
        right_child_count: child_count,
        collapsible: child_count > 0,
    });

    // Recurse into children for added containers
    if let Some(obj) = value.as_object() {
        for (k, v) in obj.iter() {
            let child_path = build_child_path(&path, PathSegment::Key(k));
            add_added_node(v, child_path, depth + 1, Some(k.to_string()), nodes);
        }
    } else if let Some(arr) = value.as_array() {
        for (idx, v) in arr.iter().enumerate() {
            let child_path = build_child_path(&path, PathSegment::Index(idx));
            add_added_node(v, child_path, depth + 1, Some(format!("[{}]", idx)), nodes);
        }
    }
}

fn value_to_string<V: JsonValue>(value: &V) -> Option<String> {
    value.to_display_string()
}

fn count_children<V: JsonValue>(value: &V) -> usize {
    if let Some(obj) = value.as_object() {
        obj.len()
    } else if let Some(arr) = value.as_array() {
        arr.len()
    } else {
        0
    }
}

/// Count the number of changes in a diff result
///
/// # Examples
/// ```
/// use lib_json_tree::{diff_json, count_diff_changes, DiffType};
/// use serde_json::json;
///
/// let old = json!({"a": 1});
/// let new = json!({"a": 2, "b": 3});
/// let diffs = diff_json(&old, &new);
/// let (added, removed, modified) = count_diff_changes(&diffs);
/// assert!(added >= 1);     // "b" was added
/// assert_eq!(removed, 0);
/// assert!(modified >= 1);  // "a" was modified, root object was modified
/// ```
pub fn count_diff_changes(diffs: &[DiffNode]) -> (usize, usize, usize) {
    let mut added = 0;
    let mut removed = 0;
    let mut modified = 0;

    for diff in diffs {
        match diff.diff_type {
            DiffType::Added => added += 1,
            DiffType::Removed => removed += 1,
            DiffType::Modified => modified += 1,
            DiffType::Unchanged => {}
        }
    }

    (added, removed, modified)
}

/// Filter diff nodes to show only changes (exclude unchanged nodes)
///
/// # Examples
/// ```
/// use lib_json_tree::{diff_json, filter_diff_changes};
/// use serde_json::json;
///
/// let old = json!({"a": 1, "b": 2});
/// let new = json!({"a": 1, "b": 3});
/// let diffs = diff_json(&old, &new);
/// let changes_only = filter_diff_changes(&diffs);
/// assert!(changes_only.iter().all(|d| d.diff_type != lib_json_tree::DiffType::Unchanged));
/// ```
pub fn filter_diff_changes(diffs: &[DiffNode]) -> Vec<&DiffNode> {
    diffs
        .iter()
        .filter(|d| d.diff_type != DiffType::Unchanged)
        .collect()
}

// ==================== Syntax Highlighting Hints ====================

/// A span within text that should be highlighted
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    /// Start byte offset in the text
    pub start: usize,
    /// End byte offset in the text (exclusive)
    pub end: usize,
    /// Type of syntax element for coloring
    pub kind: SyntaxKind,
}

/// Types of syntax elements for highlighting JSON
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxKind {
    /// Object key (the name before a colon)
    Key,
    /// String value (including quotes)
    StringValue,
    /// Number value
    NumberValue,
    /// Boolean value (true/false)
    BoolValue,
    /// Null value
    NullValue,
    /// Structural characters: { } [ ] : ,
    Punctuation,
}

/// Get syntax highlighting spans for a JSON tree node's display text
///
/// # Examples
/// ```
/// use lib_json_tree::{get_highlight_spans, SyntaxKind, JsonTreeNode, JsonValueType};
/// use std::borrow::Cow;
///
/// // Create a node representing a key-value pair
/// let node = JsonTreeNode {
///     path: Cow::Owned("name".to_string()),
///     depth: 1,
///     key: Some(Cow::Owned("name".to_string())),
///     value_type: JsonValueType::String,
///     value_str: Some(Cow::Owned("\"Alice\"".to_string())),
///     child_count: 0,
///     collapsible: false,
///     is_collapsed: false,
/// };
///
/// let text = "\"name\": \"Alice\"";
/// let spans = get_highlight_spans(&node, text);
/// assert!(spans.iter().any(|s| s.kind == SyntaxKind::Key));
/// assert!(spans.iter().any(|s| s.kind == SyntaxKind::StringValue));
/// ```
pub fn get_highlight_spans(node: &JsonTreeNode, text: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();

    // If there's a key, find and highlight it
    if let Some(key) = &node.key {
        if !key.starts_with('[') {
            // Object key (not array index)
            let key_with_quotes = format!("\"{}\"", key);
            if let Some(pos) = text.find(&key_with_quotes) {
                spans.push(HighlightSpan {
                    start: pos,
                    end: pos + key_with_quotes.len(),
                    kind: SyntaxKind::Key,
                });
            }
        }
    }

    // Find colon
    if let Some(pos) = text.find(':') {
        spans.push(HighlightSpan {
            start: pos,
            end: pos + 1,
            kind: SyntaxKind::Punctuation,
        });
    }

    // Highlight value based on type
    if let Some(value_str) = &node.value_str {
        let kind = match node.value_type {
            JsonValueType::String => SyntaxKind::StringValue,
            JsonValueType::Number => SyntaxKind::NumberValue,
            JsonValueType::Bool => SyntaxKind::BoolValue,
            JsonValueType::Null => SyntaxKind::NullValue,
            _ => return spans,
        };

        let value_str_ref: &str = value_str;
        if let Some(pos) = text.rfind(value_str_ref) {
            spans.push(HighlightSpan {
                start: pos,
                end: pos + value_str.len(),
                kind,
            });
        }
    }

    // Highlight structural characters for containers
    match node.value_type {
        JsonValueType::Object => {
            if let Some(pos) = text.find('{') {
                spans.push(HighlightSpan {
                    start: pos,
                    end: pos + 1,
                    kind: SyntaxKind::Punctuation,
                });
            }
            if let Some(pos) = text.rfind('}') {
                spans.push(HighlightSpan {
                    start: pos,
                    end: pos + 1,
                    kind: SyntaxKind::Punctuation,
                });
            }
        }
        JsonValueType::Array => {
            if let Some(pos) = text.find('[') {
                spans.push(HighlightSpan {
                    start: pos,
                    end: pos + 1,
                    kind: SyntaxKind::Punctuation,
                });
            }
            if let Some(pos) = text.rfind(']') {
                spans.push(HighlightSpan {
                    start: pos,
                    end: pos + 1,
                    kind: SyntaxKind::Punctuation,
                });
            }
        }
        _ => {}
    }

    // Sort spans by start position
    spans.sort_by_key(|s| s.start);
    spans
}

/// Format a JSON tree node as display text with optional compact preview
///
/// Returns a formatted string suitable for display in a tree view.
///
/// # Examples
/// ```
/// use lib_json_tree::{format_node_display, JsonTreeNode, JsonValueType};
/// use std::borrow::Cow;
///
/// let node = JsonTreeNode {
///     path: Cow::Owned("name".to_string()),
///     depth: 1,
///     key: Some(Cow::Owned("name".to_string())),
///     value_type: JsonValueType::String,
///     value_str: Some(Cow::Owned("\"Alice\"".to_string())),
///     child_count: 0,
///     collapsible: false,
///     is_collapsed: false,
/// };
///
/// let display = format_node_display(&node);
/// assert_eq!(display, "\"name\": \"Alice\"");
/// ```
pub fn format_node_display(node: &JsonTreeNode) -> String {
    let mut result = String::new();

    // Add key if present (and not array index)
    if let Some(key) = &node.key {
        if key.starts_with('[') {
            result.push_str(key);
        } else {
            result.push_str(&format!("\"{}\"", key));
        }
        result.push_str(": ");
    }

    // Add value representation
    match node.value_type {
        JsonValueType::Object => {
            if node.is_collapsed {
                result.push_str(&format!("{{...}} ({} keys)", node.child_count));
            } else if node.child_count == 0 {
                result.push_str("{}");
            } else {
                result.push('{');
            }
        }
        JsonValueType::Array => {
            if node.is_collapsed {
                result.push_str(&format!("[...] ({} items)", node.child_count));
            } else if node.child_count == 0 {
                result.push_str("[]");
            } else {
                result.push('[');
            }
        }
        _ => {
            if let Some(val) = &node.value_str {
                result.push_str(val);
            }
        }
    }

    result
}

// ==================== Virtual Scrolling Hints ====================

/// Information about the visible range for virtual scrolling
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleRange {
    /// Index of first visible node
    pub start_index: usize,
    /// Index of last visible node (exclusive)
    pub end_index: usize,
    /// Total number of nodes
    pub total_count: usize,
    /// Number of nodes before visible range
    pub nodes_above: usize,
    /// Number of nodes after visible range
    pub nodes_below: usize,
}

impl VisibleRange {
    /// Check if a given index is within the visible range
    pub fn is_visible(&self, index: usize) -> bool {
        index >= self.start_index && index < self.end_index
    }

    /// Get the number of visible nodes
    pub fn visible_count(&self) -> usize {
        self.end_index.saturating_sub(self.start_index)
    }
}

/// Calculate the visible range for virtual scrolling
///
/// Given a scroll offset and viewport height (in number of nodes),
/// calculate which nodes should be rendered.
///
/// # Arguments
/// * `total_nodes` - Total number of nodes in the tree
/// * `scroll_offset` - Current scroll position (index of first visible node)
/// * `viewport_height` - Number of nodes that fit in the viewport
/// * `overscan` - Extra nodes to render above/below viewport for smooth scrolling
///
/// # Examples
/// ```
/// use lib_json_tree::calculate_visible_range;
///
/// let range = calculate_visible_range(100, 10, 20, 5);
/// assert_eq!(range.start_index, 5);  // Start 5 before scroll offset
/// assert_eq!(range.end_index, 35);   // 20 visible + 5 overscan above + 5 overscan below, capped
/// assert_eq!(range.nodes_above, 5);
/// assert_eq!(range.total_count, 100);
/// ```
pub fn calculate_visible_range(
    total_nodes: usize,
    scroll_offset: usize,
    viewport_height: usize,
    overscan: usize,
) -> VisibleRange {
    let start_index = scroll_offset.saturating_sub(overscan);
    let end_index = (scroll_offset + viewport_height + overscan).min(total_nodes);

    VisibleRange {
        start_index,
        end_index,
        total_count: total_nodes,
        nodes_above: start_index,
        nodes_below: total_nodes.saturating_sub(end_index),
    }
}

/// Get the visible slice of nodes for virtual scrolling
///
/// # Examples
/// ```
/// use lib_json_tree::{flatten_json, get_visible_nodes, JsonTreeState};
/// use serde_json::json;
///
/// let json = json!({"a": 1, "b": 2, "c": 3, "d": 4, "e": 5});
/// let state = JsonTreeState::new();
/// let nodes = flatten_json(&json, &state);
///
/// // Get nodes 1-3 (with no overscan)
/// let visible = get_visible_nodes(&nodes, 1, 2, 0);
/// assert_eq!(visible.len(), 2);
/// ```
pub fn get_visible_nodes<T>(
    nodes: &[T],
    scroll_offset: usize,
    viewport_height: usize,
    overscan: usize,
) -> &[T] {
    let range = calculate_visible_range(nodes.len(), scroll_offset, viewport_height, overscan);
    &nodes[range.start_index..range.end_index]
}

/// Calculate scroll position to ensure a specific node is visible
///
/// # Arguments
/// * `target_index` - Index of the node to scroll to
/// * `viewport_height` - Number of nodes that fit in viewport
/// * `current_scroll` - Current scroll offset
///
/// # Returns
/// The new scroll offset that would make the target node visible.
/// Returns `None` if the target is already visible.
///
/// # Examples
/// ```
/// use lib_json_tree::scroll_to_node;
///
/// // Node at index 50, viewport shows 20 items, currently at scroll 10
/// let new_scroll = scroll_to_node(50, 20, 10);
/// assert_eq!(new_scroll, Some(31)); // Scroll so node 50 is at bottom
///
/// // Node at index 15, viewport shows 20 items, currently at scroll 10
/// let new_scroll = scroll_to_node(15, 20, 10);
/// assert_eq!(new_scroll, None); // Already visible (10..30)
/// ```
pub fn scroll_to_node(
    target_index: usize,
    viewport_height: usize,
    current_scroll: usize,
) -> Option<usize> {
    let visible_end = current_scroll + viewport_height;

    if target_index < current_scroll {
        // Target is above viewport, scroll up
        Some(target_index)
    } else if target_index >= visible_end {
        // Target is below viewport, scroll down
        Some(target_index.saturating_sub(viewport_height.saturating_sub(1)))
    } else {
        // Already visible
        None
    }
}

/// Find the index of a node by its path
///
/// # Examples
/// ```
/// use lib_json_tree::{flatten_json, find_node_index, JsonTreeState};
/// use serde_json::json;
///
/// let json = json!({"users": [{"name": "Alice"}]});
/// let state = JsonTreeState::new();
/// let nodes = flatten_json(&json, &state);
///
/// let index = find_node_index(&nodes, "users[0].name");
/// assert!(index.is_some());
/// ```
pub fn find_node_index(nodes: &[JsonTreeNode], path: &str) -> Option<usize> {
    nodes.iter().position(|n| n.path == path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== JsonTreeState Tests ====================

    mod json_tree_state {
        use super::*;

        #[test]
        fn new_creates_empty_state() {
            let state = JsonTreeState::new();
            assert!(state.collapsed.is_empty());
        }

        #[test]
        fn default_creates_empty_state() {
            let state = JsonTreeState::default();
            assert!(state.collapsed.is_empty());
        }

        #[test]
        fn toggle_collapses_expanded_node() {
            let mut state = JsonTreeState::new();
            assert!(!state.is_collapsed("root"));
            state.toggle("root");
            assert!(state.is_collapsed("root"));
        }

        #[test]
        fn toggle_expands_collapsed_node() {
            let mut state = JsonTreeState::new();
            state.collapse("root");
            assert!(state.is_collapsed("root"));
            state.toggle("root");
            assert!(!state.is_collapsed("root"));
        }

        #[test]
        fn toggle_multiple_times() {
            let mut state = JsonTreeState::new();
            for i in 0..5 {
                state.toggle("path");
                assert_eq!(state.is_collapsed("path"), i % 2 == 0);
            }
        }

        #[test]
        fn is_collapsed_returns_false_for_unknown_path() {
            let state = JsonTreeState::new();
            assert!(!state.is_collapsed("unknown"));
            assert!(!state.is_collapsed("deeply.nested.path"));
        }

        #[test]
        fn expand_removes_from_collapsed() {
            let mut state = JsonTreeState::new();
            state.collapse("node");
            assert!(state.is_collapsed("node"));
            state.expand("node");
            assert!(!state.is_collapsed("node"));
        }

        #[test]
        fn expand_on_already_expanded_is_noop() {
            let mut state = JsonTreeState::new();
            state.expand("node");
            assert!(!state.is_collapsed("node"));
        }

        #[test]
        fn collapse_adds_to_collapsed() {
            let mut state = JsonTreeState::new();
            state.collapse("node");
            assert!(state.is_collapsed("node"));
        }

        #[test]
        fn collapse_idempotent() {
            let mut state = JsonTreeState::new();
            state.collapse("node");
            state.collapse("node");
            assert!(state.is_collapsed("node"));
            assert_eq!(state.collapsed.len(), 1);
        }

        #[test]
        fn expand_all_clears_all_collapsed() {
            let mut state = JsonTreeState::new();
            state.collapse("a");
            state.collapse("b");
            state.collapse("c.d.e");
            assert_eq!(state.collapsed.len(), 3);
            state.expand_all();
            assert!(state.collapsed.is_empty());
        }

        #[test]
        fn collapse_deep_with_flat_object() {
            let json: Value = serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
            let mut state = JsonTreeState::new();
            state.collapse_deep(&json, 1);
            // At depth 0, we have the root object. At depth 1, we have "a" and "b" (primitives).
            // No collapsible nodes at depth >= 1, so nothing should be collapsed.
            assert!(state.collapsed.is_empty());
        }

        #[test]
        fn collapse_deep_with_nested_object() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": {"c": 1}}}"#).unwrap();
            let mut state = JsonTreeState::new();
            state.collapse_deep(&json, 1);
            // At depth 1, we have "a" which is an object, so it should be collapsed.
            assert!(state.is_collapsed("a"));
        }

        #[test]
        fn collapse_deep_with_array() {
            let json: Value = serde_json::from_str(r#"[{"a": 1}, {"b": 2}]"#).unwrap();
            let mut state = JsonTreeState::new();
            state.collapse_deep(&json, 1);
            // At depth 1, array elements [0] and [1] are objects, so they should be collapsed.
            assert!(state.is_collapsed("[0]"));
            assert!(state.is_collapsed("[1]"));
        }

        #[test]
        fn collapse_deep_with_max_depth_0() {
            let json: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            let mut state = JsonTreeState::new();
            state.collapse_deep(&json, 0);
            // Root is at depth 0, so it should be collapsed (empty path).
            assert!(state.is_collapsed(""));
        }

        #[test]
        fn collapse_deep_deeply_nested() {
            let json: Value = serde_json::from_str(r#"{"l1": {"l2": {"l3": {"l4": 1}}}}"#).unwrap();
            let mut state = JsonTreeState::new();
            state.collapse_deep(&json, 2);
            // Depth 2 means l1.l2 should be collapsed
            assert!(!state.is_collapsed("l1"));
            assert!(state.is_collapsed("l1.l2"));
        }

        #[test]
        fn multiple_paths_independent() {
            let mut state = JsonTreeState::new();
            state.collapse("a");
            state.collapse("b");
            state.expand("a");
            assert!(!state.is_collapsed("a"));
            assert!(state.is_collapsed("b"));
        }
    }

    // ==================== try_parse_json Tests ====================

    mod try_parse_json_tests {
        use super::*;

        #[test]
        fn parses_simple_object() {
            let result = try_parse_json(r#"{"key": "value"}"#);
            assert!(result.is_some());
            let obj = result.unwrap();
            assert!(obj.is_object());
            assert_eq!(obj["key"], "value");
        }

        #[test]
        fn parses_simple_array() {
            let result = try_parse_json(r#"[1, 2, 3]"#);
            assert!(result.is_some());
            let arr = result.unwrap();
            assert!(arr.is_array());
            assert_eq!(arr.as_array().unwrap().len(), 3);
        }

        #[test]
        fn parses_nested_object() {
            let result = try_parse_json(r#"{"a": {"b": {"c": 123}}}"#);
            assert!(result.is_some());
            assert_eq!(result.unwrap()["a"]["b"]["c"], 123);
        }

        #[test]
        fn parses_nested_array() {
            let result = try_parse_json(r#"[[1, 2], [3, 4]]"#);
            assert!(result.is_some());
        }

        #[test]
        fn parses_mixed_structure() {
            let result = try_parse_json(r#"{"items": [1, 2, 3], "nested": {"key": true}}"#);
            assert!(result.is_some());
        }

        #[test]
        fn rejects_empty_string() {
            assert!(try_parse_json("").is_none());
        }

        #[test]
        fn rejects_single_character() {
            assert!(try_parse_json("{").is_none());
            assert!(try_parse_json("[").is_none());
            assert!(try_parse_json("a").is_none());
        }

        #[test]
        fn rejects_plain_text() {
            assert!(try_parse_json("not json").is_none());
            assert!(try_parse_json("hello world").is_none());
        }

        #[test]
        fn rejects_primitive_values() {
            // Primitives don't start with { or [
            assert!(try_parse_json("123").is_none());
            assert!(try_parse_json("true").is_none());
            assert!(try_parse_json("null").is_none());
            assert!(try_parse_json("\"string\"").is_none());
        }

        #[test]
        fn handles_whitespace() {
            let result = try_parse_json("  \n  {\"key\": \"value\"}  \n  ");
            assert!(result.is_some());
        }

        #[test]
        fn rejects_invalid_json_syntax() {
            assert!(try_parse_json("{key: value}").is_none());
            assert!(try_parse_json("{'key': 'value'}").is_none());
            assert!(try_parse_json("{\"key\": }").is_none());
        }

        #[test]
        fn parses_empty_object() {
            let result = try_parse_json("{}");
            assert!(result.is_some());
            assert!(result.unwrap().as_object().unwrap().is_empty());
        }

        #[test]
        fn parses_empty_array() {
            let result = try_parse_json("[]");
            assert!(result.is_some());
            assert!(result.unwrap().as_array().unwrap().is_empty());
        }

        #[test]
        fn parses_unicode_content() {
            let result = try_parse_json(r#"{"emoji": "🎉", "chinese": "中文"}"#);
            assert!(result.is_some());
            assert_eq!(result.unwrap()["emoji"], "🎉");
        }

        #[test]
        fn parses_escaped_characters() {
            let result = try_parse_json(r#"{"escaped": "line1\nline2\ttab"}"#);
            assert!(result.is_some());
        }

        #[test]
        fn handles_numbers() {
            let result = try_parse_json(r#"{"int": 42, "float": 3.14, "neg": -10, "exp": 1e10}"#);
            assert!(result.is_some());
        }
    }

    // ==================== count_nodes Tests ====================

    mod count_nodes_tests {
        use super::*;

        #[test]
        fn counts_empty_object_as_one() {
            let json: Value = serde_json::from_str("{}").unwrap();
            assert_eq!(count_nodes(&json), 1);
        }

        #[test]
        fn counts_empty_array_as_one() {
            let json: Value = serde_json::from_str("[]").unwrap();
            assert_eq!(count_nodes(&json), 1);
        }

        #[test]
        fn counts_primitive_as_one() {
            assert_eq!(count_nodes(&Value::Null), 1);
            assert_eq!(count_nodes(&Value::Bool(true)), 1);
            assert_eq!(count_nodes(&Value::Number(42.into())), 1);
            assert_eq!(count_nodes(&Value::String("test".into())), 1);
        }

        #[test]
        fn counts_flat_object() {
            let json: Value = serde_json::from_str(r#"{"a": 1, "b": 2, "c": 3}"#).unwrap();
            // 1 object + 3 primitives = 4
            assert_eq!(count_nodes(&json), 4);
        }

        #[test]
        fn counts_flat_array() {
            let json: Value = serde_json::from_str("[1, 2, 3, 4, 5]").unwrap();
            // 1 array + 5 primitives = 6
            assert_eq!(count_nodes(&json), 6);
        }

        #[test]
        fn counts_nested_object() {
            let json: Value = serde_json::from_str(r#"{"a": 1, "b": {"c": 2}}"#).unwrap();
            // 1 root + "a" value (1) + "b" object (1) + "c" value (1) = 4
            assert_eq!(count_nodes(&json), 4);
        }

        #[test]
        fn counts_nested_array() {
            let json: Value = serde_json::from_str("[[1, 2], [3]]").unwrap();
            // 1 root array + 2 inner arrays + 3 primitives = 6
            assert_eq!(count_nodes(&json), 6);
        }

        #[test]
        fn counts_deeply_nested() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": {"c": {"d": 1}}}}"#).unwrap();
            // 4 objects + 1 primitive = 5
            assert_eq!(count_nodes(&json), 5);
        }

        #[test]
        fn counts_complex_structure() {
            let json: Value = serde_json::from_str(
                r#"{"users": [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]}"#,
            )
            .unwrap();
            // root (1) + users array (1) + 2 user objects (2) + 4 primitives = 8
            assert_eq!(count_nodes(&json), 8);
        }
    }

    // ==================== format_preview Tests ====================

    mod format_preview_tests {
        use super::*;

        #[test]
        fn formats_object_preview() {
            let json: Value = serde_json::from_str(r#"{"a": 1, "b": 2, "c": 3}"#).unwrap();
            assert_eq!(format_preview(&json, 50), "{...} (3 keys)");
        }

        #[test]
        fn formats_empty_object_preview() {
            let json: Value = serde_json::from_str("{}").unwrap();
            assert_eq!(format_preview(&json, 50), "{...} (0 keys)");
        }

        #[test]
        fn formats_array_preview() {
            let json: Value = serde_json::from_str("[1, 2, 3, 4, 5]").unwrap();
            assert_eq!(format_preview(&json, 50), "[...] (5 items)");
        }

        #[test]
        fn formats_empty_array_preview() {
            let json: Value = serde_json::from_str("[]").unwrap();
            assert_eq!(format_preview(&json, 50), "[...] (0 items)");
        }

        #[test]
        fn formats_short_string() {
            let json = Value::String("hello".into());
            assert_eq!(format_preview(&json, 50), "\"hello\"");
        }

        #[test]
        fn formats_long_string_truncated() {
            let json = Value::String("this is a very long string".into());
            assert_eq!(format_preview(&json, 10), "\"this is a ...\"");
        }

        #[test]
        fn formats_string_at_exact_max_len() {
            let json = Value::String("exact".into());
            assert_eq!(format_preview(&json, 5), "\"exact\"");
        }

        #[test]
        fn formats_integer() {
            let json = Value::Number(42.into());
            assert_eq!(format_preview(&json, 50), "42");
        }

        #[test]
        fn formats_negative_number() {
            let json = Value::Number((-123).into());
            assert_eq!(format_preview(&json, 50), "-123");
        }

        #[test]
        fn formats_float() {
            let json: Value = serde_json::from_str("3.14159").unwrap();
            assert_eq!(format_preview(&json, 50), "3.14159");
        }

        #[test]
        fn formats_bool_true() {
            let json = Value::Bool(true);
            assert_eq!(format_preview(&json, 50), "true");
        }

        #[test]
        fn formats_bool_false() {
            let json = Value::Bool(false);
            assert_eq!(format_preview(&json, 50), "false");
        }

        #[test]
        fn formats_null() {
            let json = Value::Null;
            assert_eq!(format_preview(&json, 50), "null");
        }

        #[test]
        fn truncation_with_zero_max_len() {
            let json = Value::String("test".into());
            assert_eq!(format_preview(&json, 0), "\"...\"");
        }
    }

    // ==================== JsonValueType Tests ====================

    mod json_value_type_tests {
        use super::*;

        #[test]
        fn from_value_object() {
            let json: Value = serde_json::from_str("{}").unwrap();
            assert_eq!(JsonValueType::from_value(&json), JsonValueType::Object);
        }

        #[test]
        fn from_value_array() {
            let json: Value = serde_json::from_str("[]").unwrap();
            assert_eq!(JsonValueType::from_value(&json), JsonValueType::Array);
        }

        #[test]
        fn from_value_string() {
            let json = Value::String("test".into());
            assert_eq!(JsonValueType::from_value(&json), JsonValueType::String);
        }

        #[test]
        fn from_value_number_integer() {
            let json = Value::Number(42.into());
            assert_eq!(JsonValueType::from_value(&json), JsonValueType::Number);
        }

        #[test]
        fn from_value_number_float() {
            let json: Value = serde_json::from_str("3.14").unwrap();
            assert_eq!(JsonValueType::from_value(&json), JsonValueType::Number);
        }

        #[test]
        fn from_value_bool_true() {
            let json = Value::Bool(true);
            assert_eq!(JsonValueType::from_value(&json), JsonValueType::Bool);
        }

        #[test]
        fn from_value_bool_false() {
            let json = Value::Bool(false);
            assert_eq!(JsonValueType::from_value(&json), JsonValueType::Bool);
        }

        #[test]
        fn from_value_null() {
            let json = Value::Null;
            assert_eq!(JsonValueType::from_value(&json), JsonValueType::Null);
        }

        #[test]
        fn key_variant_exists() {
            // Key is a distinct variant not derived from Value
            let key_type = JsonValueType::Key;
            assert_eq!(key_type, JsonValueType::Key);
        }

        #[test]
        fn all_variants_are_distinct() {
            let variants = [
                JsonValueType::Object,
                JsonValueType::Array,
                JsonValueType::String,
                JsonValueType::Number,
                JsonValueType::Bool,
                JsonValueType::Null,
                JsonValueType::Key,
            ];
            for (i, v1) in variants.iter().enumerate() {
                for (j, v2) in variants.iter().enumerate() {
                    if i == j {
                        assert_eq!(v1, v2);
                    } else {
                        assert_ne!(v1, v2);
                    }
                }
            }
        }

        #[test]
        fn derives_debug() {
            let vt = JsonValueType::Object;
            let debug_str = format!("{:?}", vt);
            assert_eq!(debug_str, "Object");
        }

        #[test]
        fn derives_clone() {
            let original = JsonValueType::Array;
            let cloned = original;
            assert_eq!(original, cloned);
        }

        #[test]
        fn derives_copy() {
            let original = JsonValueType::String;
            let copied = original;
            // Both still usable, demonstrating Copy
            assert_eq!(original, copied);
        }
    }

    // ==================== flatten_json Tests ====================

    mod flatten_json_tests {
        use super::*;

        #[test]
        fn flattens_empty_object() {
            let json: Value = serde_json::from_str("{}").unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].value_type, JsonValueType::Object);
            assert_eq!(nodes[0].child_count, 0);
            assert!(!nodes[0].collapsible);
        }

        #[test]
        fn flattens_empty_array() {
            let json: Value = serde_json::from_str("[]").unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].value_type, JsonValueType::Array);
            assert_eq!(nodes[0].child_count, 0);
            assert!(!nodes[0].collapsible);
        }

        #[test]
        fn flattens_simple_object() {
            let json: Value = serde_json::from_str(r#"{"name": "test"}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            assert_eq!(nodes.len(), 2);
            // Root object
            assert_eq!(nodes[0].value_type, JsonValueType::Object);
            assert_eq!(nodes[0].depth, 0);
            assert!(nodes[0].collapsible);
            // Child string
            assert_eq!(nodes[1].value_type, JsonValueType::String);
            assert_eq!(nodes[1].depth, 1);
            assert_eq!(nodes[1].key.as_deref(), Some("name"));
            assert_eq!(nodes[1].value_str.as_deref(), Some("\"test\""));
        }

        #[test]
        fn flattens_simple_array() {
            let json: Value = serde_json::from_str("[1, 2]").unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            assert_eq!(nodes.len(), 3);
            // Root array
            assert_eq!(nodes[0].value_type, JsonValueType::Array);
            assert_eq!(nodes[0].child_count, 2);
            // First element
            assert_eq!(nodes[1].key.as_deref(), Some("[0]"));
            assert_eq!(nodes[1].value_str.as_deref(), Some("1"));
            // Second element
            assert_eq!(nodes[2].key.as_deref(), Some("[1]"));
            assert_eq!(nodes[2].value_str.as_deref(), Some("2"));
        }

        #[test]
        fn flattens_nested_object() {
            let json: Value = serde_json::from_str(r#"{"outer": {"inner": 42}}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            assert_eq!(nodes.len(), 3);
            // Root
            assert_eq!(nodes[0].path, "");
            assert_eq!(nodes[0].depth, 0);
            // Outer
            assert_eq!(nodes[1].path, "outer");
            assert_eq!(nodes[1].depth, 1);
            assert_eq!(nodes[1].value_type, JsonValueType::Object);
            // Inner
            assert_eq!(nodes[2].path, "outer.inner");
            assert_eq!(nodes[2].depth, 2);
            assert_eq!(nodes[2].value_str.as_deref(), Some("42"));
        }

        #[test]
        fn flattens_nested_array() {
            let json: Value = serde_json::from_str("[[1]]").unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            assert_eq!(nodes.len(), 3);
            // Root array
            assert_eq!(nodes[0].path, "");
            // Inner array
            assert_eq!(nodes[1].path, "[0]");
            assert_eq!(nodes[1].value_type, JsonValueType::Array);
            // Number
            assert_eq!(nodes[2].path, "[0][0]");
        }

        #[test]
        fn collapsed_node_hides_children() {
            let json: Value = serde_json::from_str(r#"{"parent": {"child": 1}}"#).unwrap();
            let mut state = JsonTreeState::new();
            state.collapse("parent");
            let nodes = flatten_json(&json, &state);
            // Root + parent (collapsed, no child shown)
            assert_eq!(nodes.len(), 2);
            assert!(nodes[1].is_collapsed);
        }

        #[test]
        fn collapsed_root_hides_all() {
            let json: Value = serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
            let mut state = JsonTreeState::new();
            state.collapse("");
            let nodes = flatten_json(&json, &state);
            // Only root
            assert_eq!(nodes.len(), 1);
            assert!(nodes[0].is_collapsed);
        }

        #[test]
        fn depth_tracking() {
            let json: Value = serde_json::from_str(r#"{"l1": {"l2": {"l3": 1}}}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            assert_eq!(nodes[0].depth, 0);
            assert_eq!(nodes[1].depth, 1);
            assert_eq!(nodes[2].depth, 2);
            assert_eq!(nodes[3].depth, 3);
        }

        #[test]
        fn path_generation_object() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": 1}}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            assert_eq!(nodes[0].path, "");
            assert_eq!(nodes[1].path, "a");
            assert_eq!(nodes[2].path, "a.b");
        }

        #[test]
        fn path_generation_array() {
            let json: Value = serde_json::from_str(r#"{"items": [1, 2]}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            assert_eq!(nodes[0].path, "");
            assert_eq!(nodes[1].path, "items");
            assert_eq!(nodes[2].path, "items[0]");
            assert_eq!(nodes[3].path, "items[1]");
        }

        #[test]
        fn all_primitive_types() {
            let json: Value =
                serde_json::from_str(r#"{"str": "text", "num": 42, "bool": true, "null": null}"#)
                    .unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);

            // Find each primitive node by key
            let str_node = nodes
                .iter()
                .find(|n| n.key.as_deref() == Some("str"))
                .unwrap();
            assert_eq!(str_node.value_type, JsonValueType::String);
            assert_eq!(str_node.value_str.as_deref(), Some("\"text\""));

            let num_node = nodes
                .iter()
                .find(|n| n.key.as_deref() == Some("num"))
                .unwrap();
            assert_eq!(num_node.value_type, JsonValueType::Number);
            assert_eq!(num_node.value_str.as_deref(), Some("42"));

            let bool_node = nodes
                .iter()
                .find(|n| n.key.as_deref() == Some("bool"))
                .unwrap();
            assert_eq!(bool_node.value_type, JsonValueType::Bool);
            assert_eq!(bool_node.value_str.as_deref(), Some("true"));

            let null_node = nodes
                .iter()
                .find(|n| n.key.as_deref() == Some("null"))
                .unwrap();
            assert_eq!(null_node.value_type, JsonValueType::Null);
            assert_eq!(null_node.value_str.as_deref(), Some("null"));
        }

        #[test]
        fn collapsible_flag_for_containers() {
            let json: Value = serde_json::from_str(r#"{"obj": {}, "arr": [], "val": 1}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);

            let obj_node = nodes
                .iter()
                .find(|n| n.key.as_deref() == Some("obj"))
                .unwrap();
            assert!(!obj_node.collapsible); // Empty object not collapsible

            let arr_node = nodes
                .iter()
                .find(|n| n.key.as_deref() == Some("arr"))
                .unwrap();
            assert!(!arr_node.collapsible); // Empty array not collapsible

            let val_node = nodes
                .iter()
                .find(|n| n.key.as_deref() == Some("val"))
                .unwrap();
            assert!(!val_node.collapsible); // Primitive never collapsible
        }

        #[test]
        fn collapsible_flag_for_non_empty_containers() {
            let json: Value = serde_json::from_str(r#"{"obj": {"a": 1}, "arr": [1]}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);

            let obj_node = nodes
                .iter()
                .find(|n| n.key.as_deref() == Some("obj"))
                .unwrap();
            assert!(obj_node.collapsible);

            let arr_node = nodes
                .iter()
                .find(|n| n.key.as_deref() == Some("arr"))
                .unwrap();
            assert!(arr_node.collapsible);
        }

        #[test]
        fn complex_mixed_structure() {
            let json: Value = serde_json::from_str(
                r#"{
                    "users": [
                        {"name": "Alice", "active": true},
                        {"name": "Bob", "active": false}
                    ],
                    "metadata": {
                        "count": 2,
                        "tags": ["a", "b"]
                    }
                }"#,
            )
            .unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);

            // Verify structure is properly flattened
            assert!(nodes.len() > 10);

            // Check some specific paths exist
            let paths: Vec<&str> = nodes.iter().map(|n| &*n.path).collect();
            assert!(paths.contains(&"users"));
            assert!(paths.contains(&"users[0]"));
            assert!(paths.contains(&"users[0].name"));
            assert!(paths.contains(&"metadata"));
            assert!(paths.contains(&"metadata.tags"));
            assert!(paths.contains(&"metadata.tags[0]"));
        }

        #[test]
        fn partial_collapse() {
            let json: Value = serde_json::from_str(r#"{"a": {"x": 1}, "b": {"y": 2}}"#).unwrap();
            let mut state = JsonTreeState::new();
            state.collapse("a");
            // Collapse only "a", "b" should still show children
            let nodes = flatten_json(&json, &state);

            let paths: Vec<&str> = nodes.iter().map(|n| &*n.path).collect();
            // "a" is collapsed, so "a.x" should not appear
            assert!(!paths.contains(&"a.x"));
            // "b" is expanded, so "b.y" should appear
            assert!(paths.contains(&"b.y"));
        }
    }

    // ==================== Integration Tests ====================

    mod integration_tests {
        use super::*;

        #[test]
        fn full_workflow() {
            // Parse JSON
            let json_str = r#"{"config": {"debug": true, "items": [1, 2, 3]}}"#;
            let json = try_parse_json(json_str).expect("should parse");

            // Count nodes
            assert_eq!(count_nodes(&json), 7); // root + config + debug + items + 3 array elements

            // Create state and flatten
            let mut state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            let initial_count = nodes.len();
            assert_eq!(initial_count, 7);

            // Collapse items array
            state.collapse("config.items");
            let nodes = flatten_json(&json, &state);
            assert_eq!(nodes.len(), 4); // root + config + debug + items (collapsed)

            // Toggle to expand
            state.toggle("config.items");
            let nodes = flatten_json(&json, &state);
            assert_eq!(nodes.len(), initial_count);
        }

        #[test]
        fn collapse_deep_then_expand_all() {
            let json: Value =
                serde_json::from_str(r#"{"a": {"b": {"c": 1}}, "d": {"e": 2}}"#).unwrap();
            let mut state = JsonTreeState::new();

            // Collapse at depth 1
            state.collapse_deep(&json, 1);
            let nodes = flatten_json(&json, &state);
            // Only root + collapsed a + collapsed d
            assert_eq!(nodes.len(), 3);

            // Expand all
            state.expand_all();
            let nodes = flatten_json(&json, &state);
            // All 6 nodes visible: root + a + a.b + a.b.c + d + d.e
            assert_eq!(nodes.len(), 6);
        }

        #[test]
        fn preview_for_all_types() {
            let json: Value = serde_json::from_str(
                r#"{"obj": {"x": 1}, "arr": [1], "str": "hello", "num": 42, "bool": true, "null": null}"#,
            )
            .unwrap();

            assert_eq!(format_preview(&json["obj"], 50), "{...} (1 keys)");
            assert_eq!(format_preview(&json["arr"], 50), "[...] (1 items)");
            assert_eq!(format_preview(&json["str"], 50), "\"hello\"");
            assert_eq!(format_preview(&json["num"], 50), "42");
            assert_eq!(format_preview(&json["bool"], 50), "true");
            assert_eq!(format_preview(&json["null"], 50), "null");
        }
    }

    // ==================== build_child_path Tests ====================

    mod build_child_path_tests {
        use super::*;

        #[test]
        fn key_at_root() {
            assert_eq!(build_child_path("", PathSegment::Key("users")), "users");
        }

        #[test]
        fn key_with_parent() {
            assert_eq!(
                build_child_path("users", PathSegment::Key("name")),
                "users.name"
            );
        }

        #[test]
        fn index_at_root() {
            assert_eq!(build_child_path("", PathSegment::Index(0)), "[0]");
        }

        #[test]
        fn index_with_parent() {
            assert_eq!(build_child_path("users", PathSegment::Index(0)), "users[0]");
        }

        #[test]
        fn nested_path() {
            let path = build_child_path("", PathSegment::Key("users"));
            let path = build_child_path(&path, PathSegment::Index(0));
            let path = build_child_path(&path, PathSegment::Key("name"));
            assert_eq!(path, "users[0].name");
        }

        #[test]
        fn array_within_array() {
            let path = build_child_path("", PathSegment::Index(0));
            let path = build_child_path(&path, PathSegment::Index(1));
            assert_eq!(path, "[0][1]");
        }

        #[test]
        fn key_after_index() {
            let path = build_child_path("[0]", PathSegment::Key("value"));
            assert_eq!(path, "[0].value");
        }
    }

    // ==================== filter_nodes Tests ====================

    mod filter_nodes_tests {
        use super::*;

        #[test]
        fn filter_by_key_case_insensitive() {
            let json: Value = serde_json::from_str(r#"{"Name": "Alice", "age": 30}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            let filtered = filter_nodes(&nodes, "name");
            assert_eq!(filtered.len(), 1);
            assert_eq!(filtered[0].key.as_deref(), Some("Name"));
        }

        #[test]
        fn filter_by_value_string() {
            let json: Value =
                serde_json::from_str(r#"{"name": "Alice", "city": "Boston"}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            let filtered = filter_nodes(&nodes, "alice");
            assert_eq!(filtered.len(), 1);
        }

        #[test]
        fn filter_by_path() {
            let json: Value = serde_json::from_str(r#"{"users": [{"name": "Alice"}]}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            let filtered = filter_nodes(&nodes, "users[0]");
            assert!(!filtered.is_empty());
        }

        #[test]
        fn filter_no_matches() {
            let json: Value = serde_json::from_str(r#"{"name": "Alice"}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            let filtered = filter_nodes(&nodes, "xyz");
            assert!(filtered.is_empty());
        }

        #[test]
        fn filter_empty_query() {
            let json: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            let filtered = filter_nodes(&nodes, "");
            // Empty query matches all (every string contains "")
            assert_eq!(filtered.len(), nodes.len());
        }

        #[test]
        fn filter_numeric_value() {
            let json: Value = serde_json::from_str(r#"{"count": 42, "value": 100}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);
            let filtered = filter_nodes(&nodes, "42");
            assert_eq!(filtered.len(), 1);
        }
    }

    // ==================== find_path Tests ====================

    mod find_path_tests {
        use super::*;

        #[test]
        fn find_string_value() {
            let json: Value =
                serde_json::from_str(r#"{"users": [{"name": "Alice"}, {"name": "Bob"}]}"#).unwrap();
            let path = find_path(&json, |v| v.as_str() == Some("Bob"));
            assert_eq!(path, Some("users[1].name".to_string()));
        }

        #[test]
        fn find_number_value() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": {"c": 42}}}"#).unwrap();
            let path = find_path(&json, |v| v.as_i64() == Some(42));
            assert_eq!(path, Some("a.b.c".to_string()));
        }

        #[test]
        fn find_root_match() {
            let json: Value = serde_json::from_str("{}").unwrap();
            let path = find_path(&json, |v| v.is_object());
            assert_eq!(path, Some("".to_string()));
        }

        #[test]
        fn find_no_match() {
            let json: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            let path = find_path(&json, |v| v.as_str() == Some("not found"));
            assert!(path.is_none());
        }

        #[test]
        fn find_boolean() {
            let json: Value = serde_json::from_str(r#"{"config": {"enabled": true}}"#).unwrap();
            let path = find_path(&json, |v| v.as_bool() == Some(true));
            assert_eq!(path, Some("config.enabled".to_string()));
        }

        #[test]
        fn find_null() {
            let json: Value = serde_json::from_str(r#"{"data": {"value": null}}"#).unwrap();
            let path = find_path(&json, Value::is_null);
            assert_eq!(path, Some("data.value".to_string()));
        }

        #[test]
        fn find_array() {
            let json: Value = serde_json::from_str(r#"{"items": [1, 2, 3]}"#).unwrap();
            let path = find_path(&json, |v| v.is_array() && v.as_array().unwrap().len() == 3);
            assert_eq!(path, Some("items".to_string()));
        }
    }

    // ==================== find_all_paths Tests ====================

    mod find_all_paths_tests {
        use super::*;

        #[test]
        fn find_all_matching_values() {
            let json: Value = serde_json::from_str(r#"{"a": 1, "b": {"c": 1}, "d": 2}"#).unwrap();
            let paths = find_all_paths(&json, |v| v.as_i64() == Some(1));
            assert_eq!(paths.len(), 2);
            assert!(paths.contains(&"a".to_string()));
            assert!(paths.contains(&"b.c".to_string()));
        }

        #[test]
        fn find_all_in_array() {
            let json: Value =
                serde_json::from_str(r#"[{"val": true}, {"val": false}, {"val": true}]"#).unwrap();
            let paths = find_all_paths(&json, |v| v.as_bool() == Some(true));
            assert_eq!(paths.len(), 2);
            assert!(paths.contains(&"[0].val".to_string()));
            assert!(paths.contains(&"[2].val".to_string()));
        }

        #[test]
        fn find_all_no_matches() {
            let json: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            let paths = find_all_paths(&json, |v| v.as_str().is_some());
            assert!(paths.is_empty());
        }

        #[test]
        fn find_all_strings() {
            let json: Value =
                serde_json::from_str(r#"{"name": "Alice", "data": {"city": "NYC"}, "count": 5}"#)
                    .unwrap();
            let paths = find_all_paths(&json, |v| v.is_string());
            assert_eq!(paths.len(), 2);
        }

        #[test]
        fn find_all_including_containers() {
            let json: Value = serde_json::from_str(r#"{"a": {}, "b": {}}"#).unwrap();
            let paths = find_all_paths(&json, |v| v.is_object());
            // Root + a + b
            assert_eq!(paths.len(), 3);
        }
    }

    // ==================== Serialization Tests ====================

    mod serialization_tests {
        use super::*;

        #[test]
        fn serialize_empty_state() {
            let state = JsonTreeState::new();
            let json = serde_json::to_string(&state).unwrap();
            assert_eq!(json, r#"{"collapsed":[]}"#);
        }

        #[test]
        fn serialize_state_with_collapsed() {
            let mut state = JsonTreeState::new();
            state.collapse("a");
            let json = serde_json::to_string(&state).unwrap();
            assert!(json.contains("\"a\""));
        }

        #[test]
        fn deserialize_empty_state() {
            let json = r#"{"collapsed":[]}"#;
            let state: JsonTreeState = serde_json::from_str(json).unwrap();
            assert!(state.collapsed.is_empty());
        }

        #[test]
        fn deserialize_state_with_collapsed() {
            let json = r#"{"collapsed":["a","b.c"]}"#;
            let state: JsonTreeState = serde_json::from_str(json).unwrap();
            assert!(state.is_collapsed("a"));
            assert!(state.is_collapsed("b.c"));
            assert!(!state.is_collapsed("d"));
        }

        #[test]
        fn roundtrip_serialization() {
            let mut original = JsonTreeState::new();
            original.collapse("users");
            original.collapse("users[0].name");
            original.collapse("config.items");

            let json = serde_json::to_string(&original).unwrap();
            let deserialized: JsonTreeState = serde_json::from_str(&json).unwrap();

            assert_eq!(original.collapsed, deserialized.collapsed);
        }
    }

    // ==================== JSON Pointer Tests ====================

    mod json_pointer_tests {
        use super::*;

        #[test]
        fn get_by_pointer_root() {
            let json: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            let result = get_by_pointer(&json, "");
            assert_eq!(result, Some(&json));
        }

        #[test]
        fn get_by_pointer_simple_key() {
            let json: Value = serde_json::from_str(r#"{"name": "Alice"}"#).unwrap();
            let result = get_by_pointer(&json, "/name");
            assert_eq!(result, Some(&Value::String("Alice".to_string())));
        }

        #[test]
        fn get_by_pointer_nested() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": {"c": 42}}}"#).unwrap();
            let result = get_by_pointer(&json, "/a/b/c");
            assert_eq!(result, Some(&Value::Number(42.into())));
        }

        #[test]
        fn get_by_pointer_array_index() {
            let json: Value = serde_json::from_str(r#"{"items": [10, 20, 30]}"#).unwrap();
            let result = get_by_pointer(&json, "/items/1");
            assert_eq!(result, Some(&Value::Number(20.into())));
        }

        #[test]
        fn get_by_pointer_nested_array() {
            let json: Value = serde_json::from_str(r#"{"users": [{"name": "Alice"}]}"#).unwrap();
            let result = get_by_pointer(&json, "/users/0/name");
            assert_eq!(result, Some(&Value::String("Alice".to_string())));
        }

        #[test]
        fn get_by_pointer_nonexistent() {
            let json: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            assert!(get_by_pointer(&json, "/nonexistent").is_none());
            assert!(get_by_pointer(&json, "/a/b").is_none());
        }

        #[test]
        fn get_by_pointer_invalid_no_leading_slash() {
            let json: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            assert!(get_by_pointer(&json, "a").is_none());
        }

        #[test]
        fn get_by_pointer_invalid_array_index() {
            let json: Value = serde_json::from_str(r#"[1, 2, 3]"#).unwrap();
            assert!(get_by_pointer(&json, "/10").is_none());
            assert!(get_by_pointer(&json, "/abc").is_none());
        }

        #[test]
        fn get_by_pointer_escaped_tilde() {
            let json: Value = serde_json::from_str(r#"{"a~b": 1}"#).unwrap();
            let result = get_by_pointer(&json, "/a~0b");
            assert_eq!(result, Some(&Value::Number(1.into())));
        }

        #[test]
        fn get_by_pointer_escaped_slash() {
            let json: Value = serde_json::from_str(r#"{"a/b": 2}"#).unwrap();
            let result = get_by_pointer(&json, "/a~1b");
            assert_eq!(result, Some(&Value::Number(2.into())));
        }

        #[test]
        fn path_to_pointer_empty() {
            assert_eq!(path_to_pointer(""), "");
        }

        #[test]
        fn path_to_pointer_simple_key() {
            assert_eq!(path_to_pointer("users"), "/users");
        }

        #[test]
        fn path_to_pointer_nested_keys() {
            assert_eq!(
                path_to_pointer("config.database.host"),
                "/config/database/host"
            );
        }

        #[test]
        fn path_to_pointer_with_array() {
            assert_eq!(path_to_pointer("users[0]"), "/users/0");
            assert_eq!(path_to_pointer("users[0].name"), "/users/0/name");
        }

        #[test]
        fn path_to_pointer_array_only() {
            assert_eq!(path_to_pointer("[0]"), "/0");
            assert_eq!(path_to_pointer("[0][1]"), "/0/1");
        }

        #[test]
        fn path_to_pointer_complex() {
            assert_eq!(
                path_to_pointer("data.items[0].nested[1].value"),
                "/data/items/0/nested/1/value"
            );
        }

        #[test]
        fn path_to_pointer_escapes_tilde() {
            assert_eq!(path_to_pointer("a~b"), "/a~0b");
        }

        #[test]
        fn path_to_pointer_escapes_slash() {
            assert_eq!(path_to_pointer("a/b"), "/a~1b");
        }

        #[test]
        fn pointer_to_path_empty() {
            assert_eq!(pointer_to_path(""), "");
        }

        #[test]
        fn pointer_to_path_simple_key() {
            assert_eq!(pointer_to_path("/users"), "users");
        }

        #[test]
        fn pointer_to_path_nested_keys() {
            assert_eq!(
                pointer_to_path("/config/database/host"),
                "config.database.host"
            );
        }

        #[test]
        fn pointer_to_path_with_array() {
            assert_eq!(pointer_to_path("/users/0"), "users[0]");
            assert_eq!(pointer_to_path("/users/0/name"), "users[0].name");
        }

        #[test]
        fn pointer_to_path_array_only() {
            assert_eq!(pointer_to_path("/0"), "[0]");
            assert_eq!(pointer_to_path("/0/1"), "[0][1]");
        }

        #[test]
        fn pointer_to_path_unescapes() {
            assert_eq!(pointer_to_path("/a~0b"), "a~b");
            assert_eq!(pointer_to_path("/a~1b"), "a/b");
        }

        #[test]
        fn roundtrip_path_pointer() {
            let paths = vec![
                "",
                "users",
                "users[0]",
                "users[0].name",
                "data.items[0].nested[1].value",
                "[0][1][2]",
            ];

            for path in paths {
                let pointer = path_to_pointer(path);
                let back = pointer_to_path(&pointer);
                assert_eq!(back, path, "Roundtrip failed for path: {}", path);
            }
        }
    }

    // ==================== Path Format Conversion Tests ====================

    mod path_format_tests {
        use super::*;

        fn create_node(path: &str) -> JsonTreeNode<'static> {
            JsonTreeNode {
                path: Cow::Owned(path.to_string()),
                depth: 0,
                key: None,
                value_type: JsonValueType::String,
                value_str: None,
                child_count: 0,
                collapsible: false,
                is_collapsed: false,
            }
        }

        #[test]
        fn to_json_pointer_empty() {
            let node = create_node("");
            assert_eq!(node.to_json_pointer(), "");
        }

        #[test]
        fn to_json_pointer_simple() {
            let node = create_node("users[0].name");
            assert_eq!(node.to_json_pointer(), "/users/0/name");
        }

        #[test]
        fn to_jq_path_empty() {
            let node = create_node("");
            assert_eq!(node.to_jq_path(), ".");
        }

        #[test]
        fn to_jq_path_simple_key() {
            let node = create_node("users");
            assert_eq!(node.to_jq_path(), ".users");
        }

        #[test]
        fn to_jq_path_nested() {
            let node = create_node("config.database.host");
            assert_eq!(node.to_jq_path(), ".config.database.host");
        }

        #[test]
        fn to_jq_path_with_array() {
            let node = create_node("users[0].name");
            assert_eq!(node.to_jq_path(), ".users[0].name");
        }

        #[test]
        fn to_jq_path_array_only() {
            let node = create_node("[0][1]");
            assert_eq!(node.to_jq_path(), ".[0][1]");
        }

        #[test]
        fn to_bracket_notation_empty() {
            let node = create_node("");
            assert_eq!(node.to_bracket_notation(), "");
        }

        #[test]
        fn to_bracket_notation_simple_key() {
            let node = create_node("users");
            assert_eq!(node.to_bracket_notation(), r#"["users"]"#);
        }

        #[test]
        fn to_bracket_notation_nested() {
            let node = create_node("config.database.host");
            assert_eq!(
                node.to_bracket_notation(),
                r#"["config"]["database"]["host"]"#
            );
        }

        #[test]
        fn to_bracket_notation_with_array() {
            let node = create_node("users[0].name");
            assert_eq!(node.to_bracket_notation(), r#"["users"][0]["name"]"#);
        }

        #[test]
        fn to_bracket_notation_array_only() {
            let node = create_node("[0][1]");
            assert_eq!(node.to_bracket_notation(), "[0][1]");
        }

        #[test]
        fn to_bracket_notation_complex() {
            let node = create_node("data.items[0].nested[1].value");
            assert_eq!(
                node.to_bracket_notation(),
                r#"["data"]["items"][0]["nested"][1]["value"]"#
            );
        }

        #[test]
        fn integration_with_flatten_json() {
            let json: Value = serde_json::from_str(r#"{"users": [{"name": "Alice"}]}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);

            let name_node = nodes.iter().find(|n| n.path == "users[0].name").unwrap();
            assert_eq!(name_node.to_json_pointer(), "/users/0/name");
            assert_eq!(name_node.to_jq_path(), ".users[0].name");
            assert_eq!(name_node.to_bracket_notation(), r#"["users"][0]["name"]"#);
        }
    }

    // ==================== Depth-Limited Flattening Tests ====================

    mod depth_limited_tests {
        use super::*;

        #[test]
        fn flatten_depth_0() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": 1}}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json_to_depth(&json, &state, 0);

            // Only root at depth 0
            assert_eq!(nodes.len(), 1);
            assert_eq!(nodes[0].path, "");
            assert!(nodes[0].is_collapsed); // Should be collapsed at depth limit
        }

        #[test]
        fn flatten_depth_1() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": 1}, "c": 2}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json_to_depth(&json, &state, 1);

            // root + a + c (but not a.b since it's at depth 2)
            assert_eq!(nodes.len(), 3);

            let paths: Vec<&str> = nodes.iter().map(|n| &*n.path).collect();
            assert!(paths.contains(&""));
            assert!(paths.contains(&"a"));
            assert!(paths.contains(&"c"));
        }

        #[test]
        fn flatten_depth_2() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": {"c": 1}}}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json_to_depth(&json, &state, 2);

            // root (0) + a (1) + b (2)
            assert_eq!(nodes.len(), 3);

            let paths: Vec<&str> = nodes.iter().map(|n| &*n.path).collect();
            assert!(paths.contains(&""));
            assert!(paths.contains(&"a"));
            assert!(paths.contains(&"a.b"));
            assert!(!paths.contains(&"a.b.c"));
        }

        #[test]
        fn flatten_depth_with_array() {
            let json: Value = serde_json::from_str(r#"{"items": [1, 2, 3]}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json_to_depth(&json, &state, 1);

            // root + items (array)
            assert_eq!(nodes.len(), 2);

            // items should be collapsed (at depth limit)
            let items_node = nodes.iter().find(|n| n.path == "items").unwrap();
            assert!(items_node.is_collapsed);
        }

        #[test]
        fn flatten_depth_respects_manual_collapse() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": 1}, "c": {"d": 2}}"#).unwrap();
            let mut state = JsonTreeState::new();
            state.collapse("a");

            let nodes = flatten_json_to_depth(&json, &state, 3);

            // a is manually collapsed, so a.b should not appear
            // c is not collapsed, so c.d should appear
            let paths: Vec<&str> = nodes.iter().map(|n| &*n.path).collect();
            assert!(!paths.contains(&"a.b"));
            assert!(paths.contains(&"c.d"));
        }

        #[test]
        fn flatten_depth_large_value() {
            let json: Value =
                serde_json::from_str(r#"{"a": {"b": {"c": {"d": {"e": 1}}}}}"#).unwrap();
            let state = JsonTreeState::new();

            // Test that depth limiting works for deeply nested JSON
            let nodes = flatten_json_to_depth(&json, &state, 10);
            assert_eq!(nodes.len(), 6); // All 6 levels

            let nodes = flatten_json_to_depth(&json, &state, 3);
            assert_eq!(nodes.len(), 4); // root + a + a.b + a.b.c
        }

        #[test]
        fn flatten_depth_primitives_always_shown() {
            // Primitives at depth limit should still be shown
            let json: Value = serde_json::from_str(r#"{"a": 1, "b": "text", "c": true}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json_to_depth(&json, &state, 1);

            // root + 3 primitives
            assert_eq!(nodes.len(), 4);
        }

        #[test]
        fn flatten_depth_empty_containers() {
            let json: Value = serde_json::from_str(r#"{"obj": {}, "arr": []}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json_to_depth(&json, &state, 2);

            // Empty containers should still appear
            assert_eq!(nodes.len(), 3);
        }

        #[test]
        fn flatten_depth_comparison_with_regular() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": 1}}"#).unwrap();
            let state = JsonTreeState::new();

            // With high enough depth, should match regular flatten
            let regular = flatten_json(&json, &state);
            let depth_limited = flatten_json_to_depth(&json, &state, 100);

            assert_eq!(regular.len(), depth_limited.len());
        }
    }

    // ==================== JsonPath Builder Tests ====================

    mod json_path_tests {
        use super::*;

        #[test]
        fn root_path_is_empty() {
            let path = JsonPath::root();
            assert_eq!(path.to_string(), "");
            assert!(path.is_empty());
            assert_eq!(path.len(), 0);
        }

        #[test]
        fn single_key() {
            let path = JsonPath::root().key("users");
            assert_eq!(path.to_string(), "users");
            assert!(!path.is_empty());
            assert_eq!(path.len(), 1);
        }

        #[test]
        fn nested_keys() {
            let path = JsonPath::root().key("config").key("database").key("host");
            assert_eq!(path.to_string(), "config.database.host");
            assert_eq!(path.len(), 3);
        }

        #[test]
        fn single_index() {
            let path = JsonPath::root().index(0);
            assert_eq!(path.to_string(), "[0]");
        }

        #[test]
        fn key_and_index() {
            let path = JsonPath::root().key("users").index(0);
            assert_eq!(path.to_string(), "users[0]");
        }

        #[test]
        fn complex_path() {
            let path = JsonPath::root().key("users").index(0).key("name");
            assert_eq!(path.to_string(), "users[0].name");
        }

        #[test]
        fn array_of_arrays() {
            let path = JsonPath::root().index(0).index(1).index(2);
            assert_eq!(path.to_string(), "[0][1][2]");
        }

        #[test]
        fn parse_empty() {
            let path = JsonPath::parse("");
            assert!(path.is_empty());
        }

        #[test]
        fn parse_simple_key() {
            let path = JsonPath::parse("users");
            assert_eq!(path.to_string(), "users");
        }

        #[test]
        fn parse_nested() {
            let path = JsonPath::parse("users[0].name");
            assert_eq!(path.to_string(), "users[0].name");
        }

        #[test]
        fn parse_array_only() {
            let path = JsonPath::parse("[0][1]");
            assert_eq!(path.to_string(), "[0][1]");
        }

        #[test]
        fn parent_of_root_is_none() {
            let path = JsonPath::root();
            assert!(path.parent().is_none());
        }

        #[test]
        fn parent_of_single_key() {
            let path = JsonPath::root().key("users");
            let parent = path.parent();
            assert!(parent.is_some());
            assert!(parent.unwrap().is_empty());
        }

        #[test]
        fn parent_of_nested() {
            let path = JsonPath::root().key("users").index(0).key("name");
            let parent = path.parent().unwrap();
            assert_eq!(parent.to_string(), "users[0]");
        }

        #[test]
        fn to_json_pointer() {
            let path = JsonPath::root().key("users").index(0).key("name");
            assert_eq!(path.to_json_pointer(), "/users/0/name");
        }

        #[test]
        fn to_jq_path() {
            let path = JsonPath::root().key("users").index(0).key("name");
            assert_eq!(path.to_jq_path(), ".users[0].name");
        }

        #[test]
        fn to_jq_path_root() {
            let path = JsonPath::root();
            assert_eq!(path.to_jq_path(), ".");
        }

        #[test]
        fn to_bracket_notation() {
            let path = JsonPath::root().key("users").index(0).key("name");
            assert_eq!(path.to_bracket_notation(), r#"["users"][0]["name"]"#);
        }

        #[test]
        fn get_value() {
            let json: Value = serde_json::from_str(r#"{"users": [{"name": "Alice"}]}"#).unwrap();
            let path = JsonPath::root().key("users").index(0).key("name");
            assert_eq!(path.get(&json), Some(&Value::String("Alice".to_string())));
        }

        #[test]
        fn get_nonexistent() {
            let json: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            let path = JsonPath::root().key("nonexistent");
            assert!(path.get(&json).is_none());
        }

        #[test]
        fn from_str() {
            let path: JsonPath = "users[0].name".into();
            assert_eq!(path.to_string(), "users[0].name");
        }

        #[test]
        fn from_string() {
            let path: JsonPath = String::from("users[0].name").into();
            assert_eq!(path.to_string(), "users[0].name");
        }

        #[test]
        fn equality() {
            let path1 = JsonPath::root().key("users").index(0);
            let path2 = JsonPath::parse("users[0]");
            assert_eq!(path1, path2);
        }

        #[test]
        fn clone() {
            let path1 = JsonPath::root().key("users").index(0);
            let path2 = path1.clone();
            assert_eq!(path1, path2);
        }
    }

    // ==================== JsonTreeState Builder Tests ====================

    mod json_tree_state_builder_tests {
        use super::*;

        #[test]
        fn build_empty_state() {
            let state = JsonTreeStateBuilder::new().build();
            assert!(state.collapsed.is_empty());
        }

        #[test]
        fn collapsed_at_depth() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": 1}, "c": 2}"#).unwrap();
            let state = JsonTreeStateBuilder::new()
                .collapsed_at_depth(&json, 1)
                .build();
            assert!(state.is_collapsed("a"));
            assert!(!state.is_collapsed("c")); // c is a primitive
        }

        #[test]
        fn collapse_single_path() {
            let state = JsonTreeStateBuilder::new().collapse_path("users").build();
            assert!(state.is_collapsed("users"));
        }

        #[test]
        fn collapse_multiple_paths() {
            let state = JsonTreeStateBuilder::new()
                .collapse_paths(["a", "b", "c"])
                .build();
            assert!(state.is_collapsed("a"));
            assert!(state.is_collapsed("b"));
            assert!(state.is_collapsed("c"));
        }

        #[test]
        fn expand_path_removes_from_collapsed() {
            let state = JsonTreeStateBuilder::new()
                .collapse_paths(["a", "b", "c"])
                .expand_path("b")
                .build();
            assert!(state.is_collapsed("a"));
            assert!(!state.is_collapsed("b"));
            assert!(state.is_collapsed("c"));
        }

        #[test]
        fn expand_multiple_paths() {
            let state = JsonTreeStateBuilder::new()
                .collapse_paths(["a", "b", "c", "d"])
                .expand_paths(["b", "c"])
                .build();
            assert!(state.is_collapsed("a"));
            assert!(!state.is_collapsed("b"));
            assert!(!state.is_collapsed("c"));
            assert!(state.is_collapsed("d"));
        }

        #[test]
        fn builder_via_state_method() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": 1}}"#).unwrap();
            let state = JsonTreeState::builder()
                .collapsed_at_depth(&json, 1)
                .build();
            assert!(state.is_collapsed("a"));
        }

        #[test]
        fn chaining() {
            let json: Value =
                serde_json::from_str(r#"{"a": {"b": 1}, "c": {"d": 2}, "e": 3}"#).unwrap();
            let state = JsonTreeStateBuilder::new()
                .collapsed_at_depth(&json, 1)
                .expand_path("a")
                .collapse_path("e")
                .build();
            assert!(!state.is_collapsed("a"));
            assert!(state.is_collapsed("c"));
            // Note: e is a primitive so collapsed_at_depth won't collapse it,
            // but collapse_path will add it
            assert!(state.is_collapsed("e"));
        }
    }

    // ==================== JsonTreeIterator Tests ====================

    mod json_tree_iterator_tests {
        use super::*;

        #[test]
        fn iterator_empty_object() {
            let json: Value = serde_json::from_str("{}").unwrap();
            let state = JsonTreeState::new();
            let nodes: Vec<_> = flatten_json_iter(&json, &state).collect();
            assert_eq!(nodes.len(), 1);
        }

        #[test]
        fn iterator_simple_object() {
            let json: Value = serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes: Vec<_> = flatten_json_iter(&json, &state).collect();
            assert_eq!(nodes.len(), 3);
        }

        #[test]
        fn iterator_nested() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": 1}}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes: Vec<_> = flatten_json_iter(&json, &state).collect();
            assert_eq!(nodes.len(), 3);
        }

        #[test]
        fn iterator_matches_flatten_json() {
            let json: Value = serde_json::from_str(
                r#"{"users": [{"name": "Alice"}, {"name": "Bob"}], "count": 2}"#,
            )
            .unwrap();
            let state = JsonTreeState::new();

            let vec_nodes = flatten_json(&json, &state);
            let iter_nodes: Vec<_> = flatten_json_iter(&json, &state).collect();

            assert_eq!(vec_nodes.len(), iter_nodes.len());
            for (v, i) in vec_nodes.iter().zip(iter_nodes.iter()) {
                assert_eq!(v.path, i.path);
                assert_eq!(v.depth, i.depth);
                assert_eq!(v.value_type, i.value_type);
            }
        }

        #[test]
        fn iterator_respects_collapsed() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": 1}, "c": 2}"#).unwrap();
            let mut state = JsonTreeState::new();
            state.collapse("a");

            let nodes: Vec<_> = flatten_json_iter(&json, &state).collect();
            let paths: Vec<_> = nodes.iter().map(|n| &*n.path).collect();

            assert!(paths.contains(&"a"));
            assert!(!paths.contains(&"a.b")); // Should be hidden
            assert!(paths.contains(&"c"));
        }

        #[test]
        fn iterator_early_termination() {
            let json: Value = serde_json::from_str(r#"{"a": 1, "b": 2, "c": 3, "d": 4}"#).unwrap();
            let state = JsonTreeState::new();

            // Only take first 2 nodes
            let nodes: Vec<_> = flatten_json_iter(&json, &state).take(2).collect();
            assert_eq!(nodes.len(), 2);
        }

        #[test]
        fn iterator_find() {
            let json: Value =
                serde_json::from_str(r#"{"users": [{"name": "Alice"}, {"name": "Bob"}]}"#).unwrap();
            let state = JsonTreeState::new();

            let node = flatten_json_iter(&json, &state).find(|n| n.path == "users[1].name");
            assert!(node.is_some());
            assert_eq!(node.unwrap().value_str.as_deref(), Some("\"Bob\""));
        }

        #[test]
        fn iterator_count() {
            let json: Value = serde_json::from_str(r#"{"a": {"b": {"c": 1}}}"#).unwrap();
            let state = JsonTreeState::new();
            let count = flatten_json_iter(&json, &state).count();
            assert_eq!(count, 4);
        }

        #[test]
        fn iterator_filter() {
            let json: Value =
                serde_json::from_str(r#"{"a": 1, "b": "text", "c": true, "d": null}"#).unwrap();
            let state = JsonTreeState::new();

            let strings: Vec<_> = flatten_json_iter(&json, &state)
                .filter(|n| n.value_type == JsonValueType::String)
                .collect();
            assert_eq!(strings.len(), 1);
            assert_eq!(strings[0].path, "b");
        }

        #[test]
        fn iterator_with_arrays() {
            let json: Value = serde_json::from_str(r#"[1, 2, 3]"#).unwrap();
            let state = JsonTreeState::new();
            let nodes: Vec<_> = flatten_json_iter(&json, &state).collect();
            assert_eq!(nodes.len(), 4); // array + 3 elements
        }
    }

    // ==================== JsonPath with JsonTreeState Tests ====================

    mod json_path_state_integration_tests {
        use super::*;

        #[test]
        fn toggle_with_json_path() {
            let mut state = JsonTreeState::new();
            let path = JsonPath::root().key("users").index(0);

            state.toggle_path(&path);
            assert!(state.is_collapsed_path(&path));

            state.toggle_path(&path);
            assert!(!state.is_collapsed_path(&path));
        }

        #[test]
        fn collapse_with_json_path() {
            let mut state = JsonTreeState::new();
            let path = JsonPath::root().key("config");

            state.collapse_path(&path);
            assert!(state.is_collapsed_path(&path));
        }

        #[test]
        fn expand_with_json_path() {
            let mut state = JsonTreeState::new();
            let path = JsonPath::root().key("config");

            state.collapse_path(&path);
            state.expand_path(&path);
            assert!(!state.is_collapsed_path(&path));
        }

        #[test]
        fn is_collapsed_path_consistency() {
            let mut state = JsonTreeState::new();
            state.collapse("users[0].name");

            let path = JsonPath::root().key("users").index(0).key("name");
            assert!(state.is_collapsed_path(&path));
            assert!(state.is_collapsed("users[0].name"));
        }
    }

    // ==================== JSON Diff Tests ====================

    mod diff_tests {
        use super::*;

        #[test]
        fn diff_identical_values() {
            let json1: Value = serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
            let json2 = json1.clone();
            let diffs = diff_json(&json1, &json2);

            for diff in &diffs {
                assert_eq!(diff.diff_type, DiffType::Unchanged);
            }
        }

        #[test]
        fn diff_added_key() {
            let json1: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            let json2: Value = serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
            let diffs = diff_json(&json1, &json2);

            let added = diffs.iter().find(|d| d.path == "b");
            assert!(added.is_some());
            assert_eq!(added.unwrap().diff_type, DiffType::Added);
        }

        #[test]
        fn diff_removed_key() {
            let json1: Value = serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
            let json2: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            let diffs = diff_json(&json1, &json2);

            let removed = diffs.iter().find(|d| d.path == "b");
            assert!(removed.is_some());
            assert_eq!(removed.unwrap().diff_type, DiffType::Removed);
        }

        #[test]
        fn diff_modified_value() {
            let json1: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            let json2: Value = serde_json::from_str(r#"{"a": 2}"#).unwrap();
            let diffs = diff_json(&json1, &json2);

            let modified = diffs.iter().find(|d| d.path == "a");
            assert!(modified.is_some());
            assert_eq!(modified.unwrap().diff_type, DiffType::Modified);
            assert_eq!(modified.unwrap().left_value, Some("1".to_string()));
            assert_eq!(modified.unwrap().right_value, Some("2".to_string()));
        }

        #[test]
        fn diff_type_changed() {
            let json1: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            let json2: Value = serde_json::from_str(r#"{"a": "string"}"#).unwrap();
            let diffs = diff_json(&json1, &json2);

            let modified = diffs.iter().find(|d| d.path == "a");
            assert!(modified.is_some());
            let m = modified.unwrap();
            assert_eq!(m.diff_type, DiffType::Modified);
            assert_eq!(m.left_type, Some(JsonValueType::Number));
            assert_eq!(m.right_type, Some(JsonValueType::String));
        }

        #[test]
        fn diff_array_added_item() {
            let json1: Value = serde_json::from_str(r#"[1, 2]"#).unwrap();
            let json2: Value = serde_json::from_str(r#"[1, 2, 3]"#).unwrap();
            let diffs = diff_json(&json1, &json2);

            let added = diffs.iter().find(|d| d.path == "[2]");
            assert!(added.is_some());
            assert_eq!(added.unwrap().diff_type, DiffType::Added);
        }

        #[test]
        fn diff_array_removed_item() {
            let json1: Value = serde_json::from_str(r#"[1, 2, 3]"#).unwrap();
            let json2: Value = serde_json::from_str(r#"[1, 2]"#).unwrap();
            let diffs = diff_json(&json1, &json2);

            let removed = diffs.iter().find(|d| d.path == "[2]");
            assert!(removed.is_some());
            assert_eq!(removed.unwrap().diff_type, DiffType::Removed);
        }

        #[test]
        fn diff_nested_object() {
            let json1: Value = serde_json::from_str(r#"{"user": {"name": "Alice"}}"#).unwrap();
            let json2: Value = serde_json::from_str(r#"{"user": {"name": "Bob"}}"#).unwrap();
            let diffs = diff_json(&json1, &json2);

            let modified = diffs.iter().find(|d| d.path == "user.name");
            assert!(modified.is_some());
            assert_eq!(modified.unwrap().diff_type, DiffType::Modified);
        }

        #[test]
        fn count_diff_changes_works() {
            let json1: Value = serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
            let json2: Value = serde_json::from_str(r#"{"a": 1, "b": 3, "c": 4}"#).unwrap();
            let diffs = diff_json(&json1, &json2);
            let (added, removed, modified) = count_diff_changes(&diffs);

            assert_eq!(added, 1); // "c" was added
            assert_eq!(removed, 0);
            assert!(modified >= 1); // "b" was modified
        }

        #[test]
        fn filter_diff_changes_excludes_unchanged() {
            let json1: Value = serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
            let json2: Value = serde_json::from_str(r#"{"a": 1, "b": 3}"#).unwrap();
            let diffs = diff_json(&json1, &json2);
            let changes = filter_diff_changes(&diffs);

            for change in changes {
                assert_ne!(change.diff_type, DiffType::Unchanged);
            }
        }
    }

    // ==================== Syntax Highlighting Tests ====================

    mod highlight_tests {
        use super::*;

        fn create_node_with_key_value(
            key: &str,
            value_type: JsonValueType,
            value_str: Option<&str>,
        ) -> JsonTreeNode<'static> {
            JsonTreeNode {
                path: Cow::Owned(key.to_string()),
                depth: 1,
                key: Some(Cow::Owned(key.to_string())),
                value_type,
                value_str: value_str.map(|s| Cow::Owned(s.to_string())),
                child_count: 0,
                collapsible: false,
                is_collapsed: false,
            }
        }

        #[test]
        fn highlight_string_value() {
            let node = create_node_with_key_value("name", JsonValueType::String, Some("\"Alice\""));
            let text = "\"name\": \"Alice\"";
            let spans = get_highlight_spans(&node, text);

            let key_span = spans.iter().find(|s| s.kind == SyntaxKind::Key);
            assert!(key_span.is_some());

            let value_span = spans.iter().find(|s| s.kind == SyntaxKind::StringValue);
            assert!(value_span.is_some());
        }

        #[test]
        fn highlight_number_value() {
            let node = create_node_with_key_value("count", JsonValueType::Number, Some("42"));
            let text = "\"count\": 42";
            let spans = get_highlight_spans(&node, text);

            let value_span = spans.iter().find(|s| s.kind == SyntaxKind::NumberValue);
            assert!(value_span.is_some());
        }

        #[test]
        fn highlight_bool_value() {
            let node = create_node_with_key_value("active", JsonValueType::Bool, Some("true"));
            let text = "\"active\": true";
            let spans = get_highlight_spans(&node, text);

            let value_span = spans.iter().find(|s| s.kind == SyntaxKind::BoolValue);
            assert!(value_span.is_some());
        }

        #[test]
        fn highlight_null_value() {
            let node = create_node_with_key_value("empty", JsonValueType::Null, Some("null"));
            let text = "\"empty\": null";
            let spans = get_highlight_spans(&node, text);

            let value_span = spans.iter().find(|s| s.kind == SyntaxKind::NullValue);
            assert!(value_span.is_some());
        }

        #[test]
        fn highlight_colon_punctuation() {
            let node = create_node_with_key_value("name", JsonValueType::String, Some("\"Alice\""));
            let text = "\"name\": \"Alice\"";
            let spans = get_highlight_spans(&node, text);

            let punct_span = spans.iter().find(|s| s.kind == SyntaxKind::Punctuation);
            assert!(punct_span.is_some());
        }

        #[test]
        fn highlight_object_braces() {
            let node = JsonTreeNode {
                path: Cow::Borrowed(""),
                depth: 0,
                key: None,
                value_type: JsonValueType::Object,
                value_str: None,
                child_count: 2,
                collapsible: true,
                is_collapsed: false,
            };
            let text = "{}";
            let spans = get_highlight_spans(&node, text);

            let punct_spans: Vec<_> = spans
                .iter()
                .filter(|s| s.kind == SyntaxKind::Punctuation)
                .collect();
            assert_eq!(punct_spans.len(), 2); // { and }
        }

        #[test]
        fn format_node_display_key_value() {
            let node = create_node_with_key_value("name", JsonValueType::String, Some("\"Alice\""));
            let display = format_node_display(&node);
            assert_eq!(display, "\"name\": \"Alice\"");
        }

        #[test]
        fn format_node_display_collapsed_object() {
            let node = JsonTreeNode {
                path: Cow::Owned("data".to_string()),
                depth: 1,
                key: Some(Cow::Owned("data".to_string())),
                value_type: JsonValueType::Object,
                value_str: None,
                child_count: 3,
                collapsible: true,
                is_collapsed: true,
            };
            let display = format_node_display(&node);
            assert_eq!(display, "\"data\": {...} (3 keys)");
        }

        #[test]
        fn format_node_display_collapsed_array() {
            let node = JsonTreeNode {
                path: Cow::Owned("items".to_string()),
                depth: 1,
                key: Some(Cow::Owned("items".to_string())),
                value_type: JsonValueType::Array,
                value_str: None,
                child_count: 5,
                collapsible: true,
                is_collapsed: true,
            };
            let display = format_node_display(&node);
            assert_eq!(display, "\"items\": [...] (5 items)");
        }

        #[test]
        fn format_node_display_array_index() {
            let node = JsonTreeNode {
                path: Cow::Owned("[0]".to_string()),
                depth: 1,
                key: Some(Cow::Owned("[0]".to_string())),
                value_type: JsonValueType::Number,
                value_str: Some(Cow::Owned("42".to_string())),
                child_count: 0,
                collapsible: false,
                is_collapsed: false,
            };
            let display = format_node_display(&node);
            assert_eq!(display, "[0]: 42");
        }
    }

    // ==================== Virtual Scrolling Tests ====================

    mod virtual_scroll_tests {
        use super::*;

        #[test]
        fn visible_range_basic() {
            let range = calculate_visible_range(100, 10, 20, 0);
            assert_eq!(range.start_index, 10);
            assert_eq!(range.end_index, 30);
            assert_eq!(range.total_count, 100);
            assert_eq!(range.nodes_above, 10);
            assert_eq!(range.nodes_below, 70);
        }

        #[test]
        fn visible_range_with_overscan() {
            let range = calculate_visible_range(100, 10, 20, 5);
            assert_eq!(range.start_index, 5);
            assert_eq!(range.end_index, 35);
        }

        #[test]
        fn visible_range_at_start() {
            let range = calculate_visible_range(100, 0, 20, 5);
            assert_eq!(range.start_index, 0);
            assert_eq!(range.end_index, 25);
            assert_eq!(range.nodes_above, 0);
        }

        #[test]
        fn visible_range_at_end() {
            let range = calculate_visible_range(100, 90, 20, 5);
            assert_eq!(range.start_index, 85);
            assert_eq!(range.end_index, 100);
            assert_eq!(range.nodes_below, 0);
        }

        #[test]
        fn visible_range_small_list() {
            let range = calculate_visible_range(10, 0, 20, 5);
            assert_eq!(range.start_index, 0);
            assert_eq!(range.end_index, 10);
        }

        #[test]
        fn visible_range_is_visible() {
            let range = calculate_visible_range(100, 10, 20, 0);
            assert!(!range.is_visible(9));
            assert!(range.is_visible(10));
            assert!(range.is_visible(20));
            assert!(range.is_visible(29));
            assert!(!range.is_visible(30));
        }

        #[test]
        fn visible_range_visible_count() {
            let range = calculate_visible_range(100, 10, 20, 0);
            assert_eq!(range.visible_count(), 20);
        }

        #[test]
        fn get_visible_nodes_basic() {
            let nodes: Vec<i32> = (0..100).collect();
            let visible = get_visible_nodes(&nodes, 10, 20, 0);
            assert_eq!(visible.len(), 20);
            assert_eq!(visible[0], 10);
            assert_eq!(visible[19], 29);
        }

        #[test]
        fn scroll_to_node_already_visible() {
            let result = scroll_to_node(15, 20, 10);
            assert_eq!(result, None);
        }

        #[test]
        fn scroll_to_node_above() {
            let result = scroll_to_node(5, 20, 10);
            assert_eq!(result, Some(5));
        }

        #[test]
        fn scroll_to_node_below() {
            let result = scroll_to_node(50, 20, 10);
            assert_eq!(result, Some(31));
        }

        #[test]
        fn find_node_index_existing() {
            let json: Value = serde_json::from_str(r#"{"a": 1, "b": 2}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);

            let index = find_node_index(&nodes, "a");
            assert!(index.is_some());
        }

        #[test]
        fn find_node_index_not_found() {
            let json: Value = serde_json::from_str(r#"{"a": 1}"#).unwrap();
            let state = JsonTreeState::new();
            let nodes = flatten_json(&json, &state);

            let index = find_node_index(&nodes, "nonexistent");
            assert!(index.is_none());
        }
    }

    // ==================== Property-Based Tests (proptest) ====================

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        // Strategy to generate arbitrary JSON values
        fn arb_json_value() -> impl Strategy<Value = Value> {
            let leaf = prop_oneof![
                Just(Value::Null),
                any::<bool>().prop_map(Value::Bool),
                any::<i64>().prop_map(|n| Value::Number(n.into())),
                "[a-zA-Z0-9_]{0,20}".prop_map(Value::String),
            ];

            leaf.prop_recursive(
                3,  // depth
                32, // max nodes
                10, // items per collection
                |inner| {
                    prop_oneof![
                        prop::collection::vec(inner.clone(), 0..5).prop_map(Value::Array),
                        prop::collection::hash_map("[a-zA-Z_][a-zA-Z0-9_]{0,10}", inner, 0..5)
                            .prop_map(|m| Value::Object(m.into_iter().collect())),
                    ]
                },
            )
        }

        // Strategy to generate valid dot-notation paths
        fn arb_path_string() -> impl Strategy<Value = String> {
            let key_segment = "[a-zA-Z_][a-zA-Z0-9_]{0,5}";
            let index_segment = (0..10usize).prop_map(|i| format!("[{}]", i));

            let segment = prop_oneof![key_segment.prop_map(|s| s), index_segment,];

            prop::collection::vec(segment, 0..5).prop_map(|segments| {
                let mut result = String::new();
                let mut first = true;
                for seg in segments {
                    if seg.starts_with('[') {
                        result.push_str(&seg);
                    } else {
                        if !first {
                            result.push('.');
                        }
                        result.push_str(&seg);
                    }
                    first = false;
                }
                result
            })
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            // Property: count_nodes always returns positive value
            #[test]
            fn prop_count_nodes_positive(json in arb_json_value()) {
                let count = count_nodes(&json);
                prop_assert!(count >= 1, "count_nodes should always return at least 1");
            }

            // Property: flatten_json and flatten_json_small produce same results
            #[test]
            fn prop_flatten_vec_equals_smallvec(json in arb_json_value()) {
                let state = JsonTreeState::new();
                let vec_nodes = flatten_json(&json, &state);
                let smallvec_nodes = flatten_json_small(&json, &state);

                prop_assert_eq!(
                    vec_nodes.len(),
                    smallvec_nodes.len(),
                    "Vec and SmallVec flatten should produce same number of nodes"
                );

                for (v, s) in vec_nodes.iter().zip(smallvec_nodes.iter()) {
                    prop_assert_eq!(&v.path, &s.path);
                    prop_assert_eq!(v.depth, s.depth);
                    prop_assert_eq!(v.value_type, s.value_type);
                }
            }

            // Property: flatten_json_iter produces same count as flatten_json
            #[test]
            fn prop_flatten_iter_equals_vec(json in arb_json_value()) {
                let state = JsonTreeState::new();
                let vec_count = flatten_json(&json, &state).len();
                let iter_count = flatten_json_iter(&json, &state).count();

                prop_assert_eq!(
                    vec_count,
                    iter_count,
                    "Iterator and Vec flatten should produce same count"
                );
            }

            // Property: path_to_pointer and pointer_to_path are inverses (for valid paths)
            #[test]
            fn prop_path_pointer_roundtrip(path in arb_path_string()) {
                let pointer = path_to_pointer(&path);
                let back = pointer_to_path(&pointer);
                prop_assert_eq!(
                    &path, &back,
                    "path -> pointer -> path should be identity"
                );
            }

            // Property: JsonPath::parse roundtrips through to_string
            #[test]
            fn prop_json_path_roundtrip(path in arb_path_string()) {
                let parsed = JsonPath::parse(&path);
                let stringified = parsed.to_string();
                prop_assert_eq!(
                    &path, &stringified,
                    "parse -> to_string should be identity"
                );
            }

            // Property: collapse and expand are inverses
            #[test]
            fn prop_collapse_expand_inverse(path in arb_path_string()) {
                let mut state = JsonTreeState::new();

                state.collapse(&path);
                prop_assert!(state.is_collapsed(&path));

                state.expand(&path);
                prop_assert!(!state.is_collapsed(&path));
            }

            // Property: toggle is self-inverse
            #[test]
            fn prop_toggle_self_inverse(path in arb_path_string()) {
                let mut state = JsonTreeState::new();
                let initial = state.is_collapsed(&path);

                state.toggle(&path);
                state.toggle(&path);

                prop_assert_eq!(
                    initial,
                    state.is_collapsed(&path),
                    "toggle twice should return to original state"
                );
            }

            // Property: depth-limited flatten never returns nodes deeper than limit
            #[test]
            fn prop_depth_limited_respects_limit(json in arb_json_value(), max_depth in 0..5usize) {
                let state = JsonTreeState::new();
                let nodes = flatten_json_to_depth(&json, &state, max_depth);

                for node in &nodes {
                    prop_assert!(
                        node.depth <= max_depth,
                        "Node at depth {} exceeds max_depth {}",
                        node.depth,
                        max_depth
                    );
                }
            }

            // Property: filter_nodes returns subset of original nodes
            #[test]
            fn prop_filter_returns_subset(json in arb_json_value(), query in "[a-z]{0,3}") {
                let state = JsonTreeState::new();
                let all_nodes = flatten_json(&json, &state);
                let filtered = filter_nodes(&all_nodes, &query);

                prop_assert!(
                    filtered.len() <= all_nodes.len(),
                    "Filtered nodes should be <= total nodes"
                );
            }

            // Property: parse_json succeeds iff try_parse_json succeeds
            #[test]
            fn prop_parse_json_consistency(text in ".*") {
                let result = parse_json(&text);
                let option = try_parse_json(&text);

                prop_assert_eq!(
                    result.is_ok(),
                    option.is_some(),
                    "parse_json and try_parse_json should agree"
                );
            }

            // Property: expand_all clears all collapsed state
            #[test]
            fn prop_expand_all_clears(paths in prop::collection::vec(arb_path_string(), 0..10)) {
                let mut state = JsonTreeState::new();

                for path in &paths {
                    state.collapse(path);
                }

                state.expand_all();

                for path in &paths {
                    prop_assert!(
                        !state.is_collapsed(path),
                        "expand_all should clear all collapsed paths"
                    );
                }
            }
        }
    }
}
