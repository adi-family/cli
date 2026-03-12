//! Semantic syntax highlighting tokenizer - framework-agnostic
//!
//! Detects and tokenizes common patterns in command output:
//! - Strings (quoted text)
//! - Numbers
//! - Variables (env vars, assignments)
//! - File paths
//! - Key-value pairs
//! - Operators and punctuation

/// Token types for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    /// Plain text
    Plain,
    /// Quoted string
    String,
    /// Numeric value
    Number,
    /// Variable name ($VAR or VAR=)
    Variable,
    /// File path
    Path,
    /// Key in key:value or key=value
    Key,
    /// Operator or punctuation
    Operator,
    /// Comment (lines starting with #)
    Comment,
    /// Function/command name
    Function,
    /// Boolean value (true/false)
    Boolean,
    /// Special keyword (null, nil, none)
    Keyword,
    /// URL
    Url,
    /// IP address
    IpAddress,
    /// Date/time
    DateTime,
    /// Error/warning indicators
    Error,
    /// Success indicators
    Success,
}

impl TokenType {
    /// Get a semantic name for the token type
    pub fn name(&self) -> &'static str {
        match self {
            TokenType::Plain => "plain",
            TokenType::String => "string",
            TokenType::Number => "number",
            TokenType::Variable => "variable",
            TokenType::Path => "path",
            TokenType::Key => "key",
            TokenType::Operator => "operator",
            TokenType::Comment => "comment",
            TokenType::Function => "function",
            TokenType::Boolean => "boolean",
            TokenType::Keyword => "keyword",
            TokenType::Url => "url",
            TokenType::IpAddress => "ip",
            TokenType::DateTime => "datetime",
            TokenType::Error => "error",
            TokenType::Success => "success",
        }
    }
}

/// A highlighted token with text and type
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub text: String,
    pub token_type: TokenType,
}

impl Token {
    /// Create a new token
    pub fn new(text: impl Into<String>, token_type: TokenType) -> Self {
        Self {
            text: text.into(),
            token_type,
        }
    }

    /// Create a plain text token
    pub fn plain(text: impl Into<String>) -> Self {
        Self::new(text, TokenType::Plain)
    }
}

/// Parse a line of output into highlighted tokens
pub fn tokenize_line(line: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    // Check for comment line
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return vec![Token::new(line.to_string(), TokenType::Comment)];
    }

    while i < chars.len() {
        // String (double quotes)
        if chars[i] == '"' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < chars.len() {
                i += 1;
            }
            tokens.push(Token::new(
                chars[start..i].iter().collect::<String>(),
                TokenType::String,
            ));
            continue;
        }

        // String (single quotes)
        if chars[i] == '\'' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != '\'' {
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            tokens.push(Token::new(
                chars[start..i].iter().collect::<String>(),
                TokenType::String,
            ));
            continue;
        }

        // Variable ($VAR or ${VAR})
        if chars[i] == '$' {
            let start = i;
            i += 1;
            if i < chars.len() && chars[i] == '{' {
                i += 1;
                while i < chars.len() && chars[i] != '}' {
                    i += 1;
                }
                if i < chars.len() {
                    i += 1;
                }
            } else {
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
            }
            tokens.push(Token::new(
                chars[start..i].iter().collect::<String>(),
                TokenType::Variable,
            ));
            continue;
        }

        // Number
        if chars[i].is_ascii_digit()
            || (chars[i] == '-' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            if chars[i] == '-' {
                i += 1;
            }
            while i < chars.len()
                && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '_')
            {
                i += 1;
            }
            // Check for units (KB, MB, ms, etc.)
            while i < chars.len() && chars[i].is_alphabetic() {
                i += 1;
            }
            tokens.push(Token::new(
                chars[start..i].iter().collect::<String>(),
                TokenType::Number,
            ));
            continue;
        }

        // File path (starts with / or ./ or ~/)
        if chars[i] == '/'
            || (chars[i] == '.' && i + 1 < chars.len() && chars[i + 1] == '/')
            || (chars[i] == '~' && i + 1 < chars.len() && chars[i + 1] == '/')
        {
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() && chars[i] != ':' && chars[i] != ','
            {
                i += 1;
            }
            tokens.push(Token::new(
                chars[start..i].iter().collect::<String>(),
                TokenType::Path,
            ));
            continue;
        }

        // Word (check for key:value, VAR=value, keywords, booleans, URLs)
        if chars[i].is_alphanumeric() || chars[i] == '_' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric()
                    || chars[i] == '_'
                    || chars[i] == '-'
                    || chars[i] == '.'
                    || chars[i] == ':'
                    || chars[i] == '/')
            {
                // Check for URL
                if i > start + 3 {
                    let partial: String = chars[start..i].iter().collect();
                    if partial.starts_with("http://") || partial.starts_with("https://") {
                        // Continue until whitespace
                        while i < chars.len() && !chars[i].is_whitespace() {
                            i += 1;
                        }
                        tokens.push(Token::new(
                            chars[start..i].iter().collect::<String>(),
                            TokenType::Url,
                        ));
                        break;
                    }
                }
                i += 1;
            }

            if tokens.last().map(|t| t.token_type) == Some(TokenType::Url) {
                continue;
            }

            let word: String = chars[start..i].iter().collect();
            let word_lower = word.to_lowercase();

            // Check for key=value pattern
            if i < chars.len() && chars[i] == '=' {
                tokens.push(Token::new(word, TokenType::Key));
                continue;
            }

            // Check for key: pattern (but not URL schemes)
            if i < chars.len() && chars[i] == ':' && !word_lower.starts_with("http") {
                tokens.push(Token::new(word, TokenType::Key));
                continue;
            }

            // Check for boolean
            if word_lower == "true" || word_lower == "false" {
                tokens.push(Token::new(word, TokenType::Boolean));
                continue;
            }

            // Check for null/nil/none
            if word_lower == "null" || word_lower == "nil" || word_lower == "none" {
                tokens.push(Token::new(word, TokenType::Keyword));
                continue;
            }

            // Check for error/warning keywords
            if word_lower == "error"
                || word_lower == "err"
                || word_lower == "fail"
                || word_lower == "failed"
                || word_lower == "fatal"
            {
                tokens.push(Token::new(word, TokenType::Error));
                continue;
            }

            // Check for success keywords
            if word_lower == "ok"
                || word_lower == "success"
                || word_lower == "passed"
                || word_lower == "done"
            {
                tokens.push(Token::new(word, TokenType::Success));
                continue;
            }

            // Check if it looks like a function/command (if at start of line or after pipe)
            let is_first = tokens.is_empty()
                || tokens
                    .iter()
                    .all(|t| matches!(t.token_type, TokenType::Operator));
            if is_first
                && word
                    .chars()
                    .all(|c| c.is_lowercase() || c == '_' || c == '-')
            {
                tokens.push(Token::new(word, TokenType::Function));
                continue;
            }

            tokens.push(Token::new(word, TokenType::Plain));
            continue;
        }

        // Operator or punctuation
        if "=:{}[]().,;|&<>!+-*/%".contains(chars[i]) {
            tokens.push(Token::new(chars[i].to_string(), TokenType::Operator));
            i += 1;
            continue;
        }

        // Whitespace or other - keep as plain
        let start = i;
        while i < chars.len()
            && !chars[i].is_alphanumeric()
            && !"$\"'=:{}[]().,;|&<>!+-*/%_/~".contains(chars[i])
        {
            i += 1;
        }
        if start < i {
            tokens.push(Token::new(
                chars[start..i].iter().collect::<String>(),
                TokenType::Plain,
            ));
        } else {
            // Fallback - advance by one
            tokens.push(Token::new(chars[i].to_string(), TokenType::Plain));
            i += 1;
        }
    }

    tokens
}

/// Tokenize multiple lines
pub fn tokenize(text: &str) -> Vec<Vec<Token>> {
    text.lines().map(tokenize_line).collect()
}

/// Merge adjacent tokens of the same type
pub fn merge_tokens(tokens: Vec<Token>) -> Vec<Token> {
    let mut result: Vec<Token> = Vec::new();
    for token in tokens {
        if let Some(last) = result.last_mut() {
            if last.token_type == token.token_type {
                last.text.push_str(&token.text);
                continue;
            }
        }
        result.push(token);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_string() {
        let tokens = tokenize_line("echo \"hello world\"");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::String));
    }

    #[test]
    fn test_tokenize_variable() {
        let tokens = tokenize_line("$HOME/projects");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Variable));
    }

    #[test]
    fn test_tokenize_number() {
        let tokens = tokenize_line("size: 42KB");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Number));
    }

    #[test]
    fn test_tokenize_path() {
        let tokens = tokenize_line("/usr/bin/cargo");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Path));
    }

    #[test]
    fn test_tokenize_key_value() {
        let tokens = tokenize_line("name=test");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Key));
    }

    #[test]
    fn test_tokenize_comment() {
        let tokens = tokenize_line("# this is a comment");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Comment);
    }

    #[test]
    fn test_tokenize_boolean() {
        let tokens = tokenize_line("enabled: true");
        assert!(tokens.iter().any(|t| t.token_type == TokenType::Boolean));
    }
}
