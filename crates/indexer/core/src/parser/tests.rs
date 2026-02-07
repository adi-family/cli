// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Parser tests.
//!
//! Note: Most parsing tests require language plugins to be installed.
//! Tests marked with #[ignore] need plugins (adi-lang-*) to run.
//! Run with: cargo test -- --ignored

#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::parser::treesitter::TreeSitterParser;
    use crate::parser::Parser;
    use crate::types::Language;

    fn parser_without_plugins() -> TreeSitterParser {
        TreeSitterParser::without_plugins()
    }

    // Without plugins, no language is supported
    #[test]
    fn test_without_plugins_no_rust_support() {
        let p = parser_without_plugins();
        assert!(!p.supports(Language::Rust));
    }

    #[test]
    fn test_without_plugins_no_python_support() {
        let p = parser_without_plugins();
        assert!(!p.supports(Language::Python));
    }

    #[test]
    fn test_without_plugins_no_javascript_support() {
        let p = parser_without_plugins();
        assert!(!p.supports(Language::JavaScript));
    }

    #[test]
    fn test_without_plugins_no_typescript_support() {
        let p = parser_without_plugins();
        assert!(!p.supports(Language::TypeScript));
    }

    #[test]
    fn test_without_plugins_unknown_unsupported() {
        let p = parser_without_plugins();
        assert!(!p.supports(Language::Unknown));
    }

    #[test]
    fn test_parse_fails_without_plugins() {
        let p = parser_without_plugins();
        let source = "fn main() {}";
        let result = p.parse(source, Language::Rust);
        assert!(result.is_err());
    }

    // Integration tests requiring plugins
    // Run with: cargo test -- --ignored

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_parse_rust_function() {
        // This test requires the adi-lang-rust plugin to be installed
        // To run: cargo test test_parse_rust_function -- --ignored
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_parse_rust_struct() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_parse_rust_enum() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_parse_rust_trait() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_parse_rust_impl() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_parse_rust_module() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_parse_rust_const() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-python plugin"]
    fn test_parse_python_function() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-python plugin"]
    fn test_parse_python_class() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-typescript plugin"]
    fn test_parse_javascript_function() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-typescript plugin"]
    fn test_parse_javascript_class() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-typescript plugin"]
    fn test_parse_typescript_interface() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-go plugin"]
    fn test_parse_go_function() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires language plugin"]
    fn test_parse_java_class() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_parse_empty_source() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_parse_comments_only() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_symbol_location() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_doc_comment_extraction() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_identifier_references() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_no_definition_as_reference() {
        unimplemented!("Run with plugins installed");
    }

    #[test]
    #[ignore = "requires adi-lang-rust plugin"]
    fn test_constant_reference_tracking() {
        unimplemented!("Run with plugins installed");
    }
}
