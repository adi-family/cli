//! Syntax Highlighting
//!
//! Provides token-based syntax highlighting for common programming languages.

/// Token types for syntax highlighting
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenType {
    /// Regular text
    Normal,
    /// Keywords (fn, let, if, else, etc.)
    Keyword,
    /// Types (String, i32, bool, etc.)
    Type,
    /// Functions and method calls
    Function,
    /// String literals
    String,
    /// Character literals
    Char,
    /// Number literals
    Number,
    /// Comments
    Comment,
    /// Operators (+, -, *, /, =, etc.)
    Operator,
    /// Punctuation (brackets, semicolons, etc.)
    Punctuation,
    /// Macros (println!, vec!, etc.)
    Macro,
    /// Attributes (#[derive], @decorator, etc.)
    Attribute,
    /// Constants and static values
    Constant,
    /// Line numbers (special)
    LineNumber,
}

/// A highlighted token with position and type
#[derive(Clone, Debug)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub token_type: TokenType,
}

/// Color theme for syntax highlighting (RGBA)
#[derive(Clone, Copy)]
pub struct Theme {
    pub normal: [f32; 4],
    pub keyword: [f32; 4],
    pub type_name: [f32; 4],
    pub function: [f32; 4],
    pub string: [f32; 4],
    pub char: [f32; 4],
    pub number: [f32; 4],
    pub comment: [f32; 4],
    pub operator: [f32; 4],
    pub punctuation: [f32; 4],
    pub macro_name: [f32; 4],
    pub attribute: [f32; 4],
    pub constant: [f32; 4],
    pub line_number: [f32; 4],
    pub background: [f32; 4],
    pub cursor: [f32; 4],
    pub selection: [f32; 4],
}

impl Theme {
    /// One Dark theme (similar to VS Code)
    pub fn one_dark() -> Self {
        Theme {
            normal: [0.67, 0.72, 0.78, 1.0],      // #ABB2BF - light gray
            keyword: [0.77, 0.45, 0.75, 1.0],     // #C678DD - purple
            type_name: [0.90, 0.73, 0.47, 1.0],   // #E5C07B - yellow
            function: [0.38, 0.68, 0.93, 1.0],    // #61AFEF - blue
            string: [0.60, 0.76, 0.45, 1.0],      // #98C379 - green
            char: [0.60, 0.76, 0.45, 1.0],        // #98C379 - green
            number: [0.82, 0.57, 0.44, 1.0],      // #D19A66 - orange
            comment: [0.38, 0.42, 0.47, 1.0],     // #5C6370 - dark gray
            operator: [0.34, 0.76, 0.83, 1.0],    // #56B6C2 - cyan
            punctuation: [0.67, 0.72, 0.78, 1.0], // #ABB2BF - light gray
            macro_name: [0.38, 0.68, 0.93, 1.0],  // #61AFEF - blue
            attribute: [0.77, 0.45, 0.75, 1.0],   // #C678DD - purple
            constant: [0.82, 0.57, 0.44, 1.0],    // #D19A66 - orange
            line_number: [0.38, 0.42, 0.47, 1.0], // #5C6370 - dark gray
            background: [0.16, 0.17, 0.20, 1.0],  // #282C34 - dark
            cursor: [1.0, 1.0, 1.0, 0.9],         // white
            selection: [0.26, 0.35, 0.50, 0.5],   // blue transparent
        }
    }

    /// Get color for a token type
    pub fn color_for(&self, token_type: TokenType) -> [f32; 4] {
        match token_type {
            TokenType::Normal => self.normal,
            TokenType::Keyword => self.keyword,
            TokenType::Type => self.type_name,
            TokenType::Function => self.function,
            TokenType::String => self.string,
            TokenType::Char => self.char,
            TokenType::Number => self.number,
            TokenType::Comment => self.comment,
            TokenType::Operator => self.operator,
            TokenType::Punctuation => self.punctuation,
            TokenType::Macro => self.macro_name,
            TokenType::Attribute => self.attribute,
            TokenType::Constant => self.constant,
            TokenType::LineNumber => self.line_number,
        }
    }
}

/// Simple syntax highlighter
pub struct Highlighter {
    pub theme: Theme,
}

impl Highlighter {
    pub fn new() -> Self {
        Highlighter {
            theme: Theme::one_dark(),
        }
    }

    /// Highlight a single line and return tokens
    pub fn highlight_line(&self, line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = line.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let start = i;
            let ch = chars[i];

            // Skip whitespace
            if ch.is_whitespace() {
                i += 1;
                continue;
            }

            // Comments (// or #)
            if ch == '/' && i + 1 < len && chars[i + 1] == '/' {
                tokens.push(Token {
                    start,
                    end: len,
                    token_type: TokenType::Comment,
                });
                break;
            }
            if ch == '#' && (i == 0 || chars[i - 1].is_whitespace()) {
                // Could be attribute or comment
                if i + 1 < len && chars[i + 1] == '[' {
                    // Rust attribute #[...]
                    let mut depth = 0;
                    while i < len {
                        if chars[i] == '[' {
                            depth += 1;
                        } else if chars[i] == ']' {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        i += 1;
                    }
                    tokens.push(Token {
                        start,
                        end: i,
                        token_type: TokenType::Attribute,
                    });
                    continue;
                } else {
                    // Python/shell comment
                    tokens.push(Token {
                        start,
                        end: len,
                        token_type: TokenType::Comment,
                    });
                    break;
                }
            }

            // String literals
            if ch == '"' || ch == '\'' || ch == '`' {
                let quote = ch;
                i += 1;
                while i < len && chars[i] != quote {
                    if chars[i] == '\\' && i + 1 < len {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i < len {
                    i += 1; // closing quote
                }
                tokens.push(Token {
                    start,
                    end: i,
                    token_type: if quote == '\'' && i - start <= 4 {
                        TokenType::Char
                    } else {
                        TokenType::String
                    },
                });
                continue;
            }

            // Numbers
            if ch.is_ascii_digit() || (ch == '.' && i + 1 < len && chars[i + 1].is_ascii_digit()) {
                while i < len
                    && (chars[i].is_ascii_alphanumeric()
                        || chars[i] == '.'
                        || chars[i] == '_'
                        || chars[i] == 'x'
                        || chars[i] == 'b'
                        || chars[i] == 'o')
                {
                    i += 1;
                }
                tokens.push(Token {
                    start,
                    end: i,
                    token_type: TokenType::Number,
                });
                continue;
            }

            // Identifiers and keywords
            if ch.is_alphabetic() || ch == '_' {
                while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }

                // Check for macro (ends with !)
                let is_macro = i < len && chars[i] == '!';
                if is_macro {
                    i += 1;
                }

                let word: String = chars[start..if is_macro { i - 1 } else { i }]
                    .iter()
                    .collect();

                let token_type = if is_macro {
                    TokenType::Macro
                } else if Self::is_keyword(&word) {
                    TokenType::Keyword
                } else if Self::is_type(&word) {
                    TokenType::Type
                } else if Self::is_constant(&word) {
                    TokenType::Constant
                } else if i < len && chars[i] == '(' {
                    TokenType::Function
                } else {
                    TokenType::Normal
                };

                tokens.push(Token {
                    start,
                    end: i,
                    token_type,
                });
                continue;
            }

            // Operators
            if "+-*/%=<>!&|^~?:".contains(ch) {
                while i < len && "+-*/%=<>!&|^~?:".contains(chars[i]) {
                    i += 1;
                }
                tokens.push(Token {
                    start,
                    end: i,
                    token_type: TokenType::Operator,
                });
                continue;
            }

            // Punctuation
            if "()[]{},.;@".contains(ch) {
                i += 1;
                tokens.push(Token {
                    start,
                    end: i,
                    token_type: TokenType::Punctuation,
                });
                continue;
            }

            // Default: single character as normal
            i += 1;
            tokens.push(Token {
                start,
                end: i,
                token_type: TokenType::Normal,
            });
        }

        tokens
    }

    fn is_keyword(word: &str) -> bool {
        matches!(
            word,
            // Rust keywords
            "fn" | "let" | "mut" | "const" | "static" | "if" | "else" | "match" | "loop"
            | "while" | "for" | "in" | "break" | "continue" | "return" | "struct" | "enum"
            | "impl" | "trait" | "type" | "where" | "pub" | "mod" | "use" | "crate" | "self"
            | "Self" | "super" | "async" | "await" | "move" | "ref" | "dyn" | "unsafe"
            | "extern" | "as" | "true" | "false"
            // JS/TS keywords
            | "function" | "var" | "class" | "extends" | "new" | "this" | "import" | "export"
            | "from" | "default" | "try" | "catch" | "finally" | "throw" | "typeof" | "instanceof"
            | "delete" | "void" | "yield" | "interface" | "declare" | "readonly" | "private"
            | "protected" | "public" | "abstract" | "implements" | "package"
            // Python keywords
            | "def" | "class" | "if" | "elif" | "else" | "for" | "while" | "try" | "except"
            | "finally" | "with" | "as" | "import" | "from" | "return" | "yield" | "raise"
            | "pass" | "break" | "continue" | "and" | "or" | "not" | "is" | "in" | "lambda"
            | "global" | "nonlocal" | "assert" | "del" | "True" | "False" | "None"
            // Go keywords
            | "func" | "package" | "import" | "var" | "const" | "type" | "struct" | "interface"
            | "map" | "chan" | "go" | "select" | "case" | "default" | "fallthrough" | "defer"
            | "range"
        )
    }

    fn is_type(word: &str) -> bool {
        // Check if starts with uppercase (common convention for types)
        let first_char = word.chars().next().unwrap_or('a');
        if first_char.is_uppercase() && word.len() > 1 {
            return true;
        }

        matches!(
            word,
            // Rust types
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
            | "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
            | "f32" | "f64" | "bool" | "char" | "str"
            | "String" | "Vec" | "Option" | "Result" | "Box" | "Rc" | "Arc"
            | "HashMap" | "HashSet" | "BTreeMap" | "BTreeSet"
            // JS/TS types  
            | "string" | "number" | "boolean" | "object" | "any" | "void" | "never"
            | "unknown" | "null" | "undefined" | "Array" | "Promise" | "Map" | "Set"
            // Python types
            | "int" | "float" | "str" | "bool" | "list" | "dict" | "tuple" | "set"
            | "bytes" | "bytearray" | "None"
        )
    }

    fn is_constant(word: &str) -> bool {
        // ALL_CAPS convention for constants
        word.len() > 1 && word.chars().all(|c| c.is_uppercase() || c == '_')
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}
