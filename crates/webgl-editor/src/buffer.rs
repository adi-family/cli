//! Text Buffer
//!
//! Uses the Rope data structure for efficient text operations on large files.
//! Rope provides O(log n) insert/delete operations instead of O(n) for strings.

use ropey::Rope;

/// Text buffer backed by a rope data structure
pub struct TextBuffer {
    rope: Rope,
}

impl TextBuffer {
    pub fn new() -> Self {
        TextBuffer { rope: Rope::new() }
    }

    /// Set the entire buffer content
    pub fn set_content(&mut self, text: &str) {
        self.rope = Rope::from_str(text);
    }

    /// Get the entire buffer content as a string
    pub fn get_content(&self) -> String {
        self.rope.to_string()
    }

    /// Get the number of lines in the buffer
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// Get a specific line by index (0-based)
    pub fn get_line(&self, line_idx: usize) -> Option<String> {
        if line_idx >= self.rope.len_lines() {
            return None;
        }

        let line = self.rope.line(line_idx);
        // Remove trailing newline if present
        let s = line.to_string();
        Some(s.trim_end_matches(&['\n', '\r'][..]).to_string())
    }

    /// Get the length of a specific line (excluding newline)
    pub fn line_len(&self, line_idx: usize) -> usize {
        if line_idx >= self.rope.len_lines() {
            return 0;
        }

        let line = self.rope.line(line_idx);
        let s = line.to_string();
        s.trim_end_matches(&['\n', '\r'][..]).len()
    }

    /// Insert text at a specific position
    pub fn insert(&mut self, line: usize, col: usize, text: &str) {
        if let Some(char_idx) = self.line_col_to_char_idx(line, col) {
            self.rope.insert(char_idx, text);
        }
    }

    /// Insert a character at a specific position
    pub fn insert_char(&mut self, line: usize, col: usize, ch: char) {
        if let Some(char_idx) = self.line_col_to_char_idx(line, col) {
            self.rope.insert_char(char_idx, ch);
        }
    }

    /// Delete a range of text
    pub fn delete_range(
        &mut self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) {
        let start_idx = self.line_col_to_char_idx(start_line, start_col);
        let end_idx = self.line_col_to_char_idx(end_line, end_col);

        if let (Some(start), Some(end)) = (start_idx, end_idx) {
            if start < end && end <= self.rope.len_chars() {
                self.rope.remove(start..end);
            }
        }
    }

    /// Delete a single character before the cursor (backspace)
    pub fn delete_char_before(&mut self, line: usize, col: usize) -> Option<(usize, usize)> {
        if col > 0 {
            // Delete character on same line
            if let Some(char_idx) = self.line_col_to_char_idx(line, col - 1) {
                self.rope.remove(char_idx..char_idx + 1);
                return Some((line, col - 1));
            }
        } else if line > 0 {
            // Delete newline, join with previous line
            let prev_line_len = self.line_len(line - 1);
            if let Some(char_idx) = self.line_col_to_char_idx(line, 0) {
                // Remove the newline character(s) before current line
                let newline_start = char_idx - 1;
                // Check for \r\n
                if newline_start > 0 {
                    let prev_char = self.rope.char(newline_start);
                    if prev_char == '\n' {
                        let remove_start =
                            if newline_start > 0 && self.rope.char(newline_start - 1) == '\r' {
                                newline_start - 1
                            } else {
                                newline_start
                            };
                        self.rope.remove(remove_start..char_idx);
                    }
                }
                return Some((line - 1, prev_line_len));
            }
        }
        None
    }

    /// Delete a single character at the cursor (delete key)
    pub fn delete_char_at(&mut self, line: usize, col: usize) -> bool {
        let line_len = self.line_len(line);

        if col < line_len {
            // Delete character on same line
            if let Some(char_idx) = self.line_col_to_char_idx(line, col) {
                self.rope.remove(char_idx..char_idx + 1);
                return true;
            }
        } else if line < self.line_count() - 1 {
            // Delete newline, join with next line
            if let Some(char_idx) = self.line_col_to_char_idx(line, col) {
                // Remove newline character(s)
                let next_char = self.rope.char(char_idx);
                let remove_end = if next_char == '\r' && char_idx + 1 < self.rope.len_chars() {
                    char_idx + 2
                } else {
                    char_idx + 1
                };
                self.rope.remove(char_idx..remove_end);
                return true;
            }
        }
        false
    }

    /// Insert a newline at the cursor position
    pub fn insert_newline(&mut self, line: usize, col: usize) {
        if let Some(char_idx) = self.line_col_to_char_idx(line, col) {
            self.rope.insert_char(char_idx, '\n');
        }
    }

    /// Get a range of text
    pub fn get_range(
        &self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) -> String {
        let start_idx = self.line_col_to_char_idx(start_line, start_col);
        let end_idx = self.line_col_to_char_idx(end_line, end_col);

        if let (Some(start), Some(end)) = (start_idx, end_idx) {
            if start <= end && end <= self.rope.len_chars() {
                return self.rope.slice(start..end).to_string();
            }
        }
        String::new()
    }

    /// Convert line/column to character index
    fn line_col_to_char_idx(&self, line: usize, col: usize) -> Option<usize> {
        if line >= self.rope.len_lines() {
            return None;
        }

        let line_start = self.rope.line_to_char(line);
        let line_len = self.line_len(line);
        let col = col.min(line_len);

        Some(line_start + col)
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Get total character count
    pub fn char_count(&self) -> usize {
        self.rope.len_chars()
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut buffer = TextBuffer::new();
        buffer.set_content("Hello\nWorld");

        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.get_line(0), Some("Hello".to_string()));
        assert_eq!(buffer.get_line(1), Some("World".to_string()));
    }

    #[test]
    fn test_insert() {
        let mut buffer = TextBuffer::new();
        buffer.set_content("Hello World");

        buffer.insert_char(0, 5, ',');
        assert_eq!(buffer.get_line(0), Some("Hello, World".to_string()));
    }

    #[test]
    fn test_delete() {
        let mut buffer = TextBuffer::new();
        buffer.set_content("Hello World");

        buffer.delete_char_at(0, 5);
        assert_eq!(buffer.get_line(0), Some("HelloWorld".to_string()));
    }
}
