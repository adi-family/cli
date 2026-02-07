use crate::ToolFlag;
use regex::Regex;

/// Parse --help text to extract examples and flags
pub fn parse_help_text(help: &str) -> (Vec<String>, Vec<ToolFlag>) {
    let examples = extract_examples(help);
    let flags = extract_flags(help);
    (examples, flags)
}

fn extract_examples(help: &str) -> Vec<String> {
    let mut examples = Vec::new();
    let mut in_examples = false;

    for line in help.lines() {
        let trimmed = line.trim();

        // Detect example sections
        if trimmed.eq_ignore_ascii_case("examples:")
            || trimmed.eq_ignore_ascii_case("example:")
            || trimmed.starts_with("Examples:")
            || trimmed.starts_with("EXAMPLES:")
        {
            in_examples = true;
            continue;
        }

        // End of examples section (new section header)
        if in_examples
            && !trimmed.is_empty()
            && !trimmed.starts_with(' ')
            && !trimmed.starts_with('$')
            && !trimmed.starts_with('#')
            && trimmed.ends_with(':') && !trimmed.contains(' ') {
                in_examples = false;
                continue;
            }

        if in_examples && !trimmed.is_empty() {
            // Detect command lines with shell prompt
            if trimmed.starts_with('$') || trimmed.starts_with('%') {
                examples.push(trimmed[1..].trim().to_string());
            } else if trimmed.starts_with('#') {
                // Comment line, skip
                continue;
            } else if line.starts_with("  ") && !line.starts_with("    ") {
                // Indented example without shell prompt (2 spaces, not 4)
                examples.push(trimmed.to_string());
            } else if line.starts_with("    ") && !trimmed.starts_with('-') {
                // 4-space indent, but not a flag description
                examples.push(trimmed.to_string());
            }
        }
    }

    examples
}

fn extract_flags(help: &str) -> Vec<ToolFlag> {
    let mut flags = Vec::new();

    // Match patterns like:
    //   -h, --help              Show help
    //   --verbose               Enable verbose mode
    //   -o, --output <FILE>     Output file
    //       --config <PATH>     Config path
    let flag_regex = Regex::new(
        r"^\s{2,8}(-[a-zA-Z])?,?\s*(--[a-zA-Z][-a-zA-Z0-9]*)?(?:\s+[<\[][^>\]]+[>\]])?\s{2,}(.+)",
    )
    .unwrap();

    // Also match single dash flags without long version
    let short_only_regex =
        Regex::new(r"^\s{2,8}(-[a-zA-Z])\s+[<\[][^>\]]+[>\]]?\s{2,}(.+)").unwrap();

    for line in help.lines() {
        if let Some(caps) = flag_regex.captures(line) {
            let short = caps.get(1).map(|m| m.as_str().to_string());
            let long = caps.get(2).map(|m| m.as_str().to_string());
            let description = caps
                .get(3)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();

            if short.is_some() || long.is_some() {
                let takes_value = line.contains('<') || line.contains('[');

                flags.push(ToolFlag {
                    short,
                    long,
                    description,
                    takes_value,
                });
            }
        } else if let Some(caps) = short_only_regex.captures(line) {
            let short = caps.get(1).map(|m| m.as_str().to_string());
            let description = caps
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();

            if short.is_some() {
                flags.push(ToolFlag {
                    short,
                    long: None,
                    description,
                    takes_value: true,
                });
            }
        }
    }

    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_flags() {
        let help = r#"
Usage: mytool [OPTIONS]

Options:
  -h, --help              Show help
  -v, --verbose           Enable verbose mode
  -o, --output <FILE>     Output file
      --config <PATH>     Config path
"#;

        let flags = extract_flags(help);
        assert_eq!(flags.len(), 4);
        assert_eq!(flags[0].short, Some("-h".to_string()));
        assert_eq!(flags[0].long, Some("--help".to_string()));
        assert!(!flags[0].takes_value);
        assert!(flags[2].takes_value);
        assert_eq!(flags[3].short, None);
        assert_eq!(flags[3].long, Some("--config".to_string()));
    }

    #[test]
    fn test_extract_examples() {
        let help = r#"
Usage: mytool [OPTIONS]

Examples:
  $ mytool --verbose
  $ mytool -o output.txt input.txt

Options:
  -h, --help  Show help
"#;

        let examples = extract_examples(help);
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0], "mytool --verbose");
        assert_eq!(examples[1], "mytool -o output.txt input.txt");
    }

    #[test]
    fn test_extract_examples_with_comments() {
        let help = r#"
Examples:
  # List all items
  $ mytool list

  # Create new item
  $ mytool create --name foo
"#;

        let examples = extract_examples(help);
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0], "mytool list");
        assert_eq!(examples[1], "mytool create --name foo");
    }

    #[test]
    fn test_parse_help_text() {
        let help = r#"
mytool - A utility for testing

Usage: mytool [OPTIONS] <COMMAND>

Options:
  -h, --help     Show help
  -V, --version  Show version

Examples:
  $ mytool run
  $ mytool build --release
"#;

        let (examples, flags) = parse_help_text(help);
        assert_eq!(examples.len(), 2);
        assert_eq!(flags.len(), 2);
    }
}
