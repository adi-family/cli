use std::path::{Path, PathBuf};

use syn::{visit::Visit, ItemFn, ImplItemFn};

/// A code chunk extracted from source: one function or method.
#[derive(Debug, Clone)]
pub struct CodeChunk {
    /// Crate name (derived from closest Cargo.toml)
    pub crate_name: String,
    /// Relative file path
    pub file_path: PathBuf,
    /// Function/method name
    pub name: String,
    /// Full source text of the function body
    pub source: String,
    /// Line number in original file
    pub line: usize,
    /// Optional parent impl type (e.g. "MyStruct" for `impl MyStruct { fn foo() }`)
    pub parent_type: Option<String>,
}

/// Visitor that collects functions and impl methods from a Rust file.
struct ChunkCollector {
    source_text: String,
    chunks: Vec<CodeChunk>,
    current_impl_type: Option<String>,
    crate_name: String,
    file_path: PathBuf,
}

impl ChunkCollector {
    /// Extract source text for a span range using line numbers.
    fn extract_source(&self, start_line: usize, end_line: usize) -> String {
        let lines: Vec<&str> = self.source_text.lines().collect();
        if start_line == 0 || end_line == 0 || start_line > lines.len() {
            return String::new();
        }
        let start = start_line.saturating_sub(1);
        let end = end_line.min(lines.len());
        lines[start..end].join("\n")
    }
}

impl<'ast> Visit<'ast> for ChunkCollector {
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        let type_name = extract_type_name(&node.self_ty);
        self.current_impl_type = Some(type_name);
        syn::visit::visit_item_impl(self, node);
        self.current_impl_type = None;
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let fn_start = node.sig.fn_token.span.start().line;
        let body_end = node.block.brace_token.span.close().start().line;
        let source = self.extract_source(fn_start, body_end);

        if count_meaningful_lines(&source) >= MIN_FUNCTION_LINES {
            self.chunks.push(CodeChunk {
                crate_name: self.crate_name.clone(),
                file_path: self.file_path.clone(),
                name: node.sig.ident.to_string(),
                source,
                line: fn_start,
                parent_type: None,
            });
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        let fn_start = node.sig.fn_token.span.start().line;
        let body_end = node.block.brace_token.span.close().start().line;
        let source = self.extract_source(fn_start, body_end);

        if count_meaningful_lines(&source) >= MIN_FUNCTION_LINES {
            self.chunks.push(CodeChunk {
                crate_name: self.crate_name.clone(),
                file_path: self.file_path.clone(),
                name: node.sig.ident.to_string(),
                source,
                line: fn_start,
                parent_type: self.current_impl_type.clone(),
            });
        }
        syn::visit::visit_impl_item_fn(self, node);
    }
}

/// Minimum lines for a function to be worth embedding.
const MIN_FUNCTION_LINES: usize = 5;

fn count_meaningful_lines(source: &str) -> usize {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .count()
}

fn extract_type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(tp) => tp
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_else(|| "Unknown".into()),
        _ => format!("{}", quote::quote!(#ty)),
    }
}

/// Parse a single Rust file and extract all function chunks.
pub fn parse_file(path: &Path, crate_name: &str) -> anyhow::Result<Vec<CodeChunk>> {
    let source_text = std::fs::read_to_string(path)?;
    let syntax = syn::parse_file(&source_text)?;

    let mut collector = ChunkCollector {
        source_text,
        chunks: Vec::new(),
        current_impl_type: None,
        crate_name: crate_name.to_string(),
        file_path: path.to_path_buf(),
    };

    collector.visit_file(&syntax);
    Ok(collector.chunks)
}

/// Resolve crate name from the nearest Cargo.toml above a file.
pub fn find_crate_name(file_path: &Path) -> Option<String> {
    let mut dir = file_path.parent()?;
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml).ok()?;
            let parsed: toml::Value = content.parse().ok()?;
            return parsed
                .get("package")?
                .get("name")?
                .as_str()
                .map(String::from);
        }
        dir = dir.parent()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_functions() {
        let source = r#"
fn short() {
    1
}

fn meaningful_function() {
    let x = 1;
    let y = 2;
    let z = x + y;
    println!("{}", z);
    z
}

struct Foo;

impl Foo {
    fn method_on_foo(&self) {
        let a = vec![1, 2, 3];
        let b: Vec<_> = a.iter().map(|x| x * 2).collect();
        println!("{:?}", b);
        let c = b.len();
        drop(c);
    }
}
"#;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), source).unwrap();

        let chunks = parse_file(tmp.path(), "test-crate").unwrap();

        // `short` has <5 meaningful lines, should be excluded
        assert!(
            !chunks.iter().any(|c| c.name == "short"),
            "short function should be filtered out"
        );

        assert!(
            chunks.iter().any(|c| c.name == "meaningful_function"),
            "meaningful_function should be extracted"
        );

        let method = chunks.iter().find(|c| c.name == "method_on_foo");
        assert!(method.is_some(), "impl method should be extracted");
        assert_eq!(
            method.unwrap().parent_type.as_deref(),
            Some("Foo"),
            "parent type should be Foo"
        );

        for chunk in &chunks {
            assert_eq!(chunk.crate_name, "test-crate");
        }
    }

    #[test]
    fn test_find_crate_name() {
        let this_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/parser.rs");
        if this_file.exists() {
            let name = find_crate_name(&this_file);
            assert_eq!(name.as_deref(), Some("experiment-semantic-duplicates"));
        }
    }

    #[test]
    fn test_count_meaningful_lines() {
        let source = r#"
    // comment
    let x = 1;

    let y = 2;
    // another comment
    let z = x + y;
"#;
        assert_eq!(count_meaningful_lines(source), 3);
    }
}
