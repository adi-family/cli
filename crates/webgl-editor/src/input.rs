//! Keyboard Input Handling
//!
//! Processes keyboard events and translates them to editor commands.

use crate::buffer::TextBuffer;
use crate::cursor::Cursor;
use crate::viewport::Viewport;

/// Handle a keyboard event
///
/// Returns true if the event was handled (should prevent default browser behavior)
pub fn handle_key(
    buffer: &mut TextBuffer,
    cursor: &mut Cursor,
    viewport: &mut Viewport,
    key: &str,
    ctrl: bool,
    shift: bool,
    _alt: bool,
) -> bool {
    let line_len = |line: usize| buffer.line_len(line);
    let line_count = buffer.line_count();

    match key {
        // Navigation keys
        "ArrowLeft" => {
            if shift {
                cursor.select_left(line_len);
            } else {
                cursor.move_left(line_len);
            }
            true
        }
        "ArrowRight" => {
            if shift {
                cursor.select_right(line_len, line_count);
            } else {
                cursor.move_right(line_len, line_count);
            }
            true
        }
        "ArrowUp" => {
            if shift {
                cursor.select_up(line_len);
            } else {
                cursor.move_up(line_len);
            }
            true
        }
        "ArrowDown" => {
            if shift {
                cursor.select_down(line_len, line_count);
            } else {
                cursor.move_down(line_len, line_count);
            }
            true
        }
        "Home" => {
            if ctrl {
                cursor.move_to_start();
            } else {
                cursor.move_to_line_start();
            }
            true
        }
        "End" => {
            if ctrl {
                let last_line_len = buffer.line_len(line_count.saturating_sub(1));
                cursor.move_to_end(line_count, last_line_len);
            } else {
                let current_line_len = buffer.line_len(cursor.line());
                cursor.move_to_line_end(current_line_len);
            }
            true
        }
        "PageUp" => {
            let visible_lines = viewport.visible_lines();
            for _ in 0..visible_lines {
                cursor.move_up(line_len);
            }
            true
        }
        "PageDown" => {
            let visible_lines = viewport.visible_lines();
            for _ in 0..visible_lines {
                cursor.move_down(line_len, line_count);
            }
            true
        }

        // Editing keys
        "Backspace" => {
            if cursor.has_selection() {
                delete_selection(buffer, cursor);
            } else if let Some((new_line, new_col)) =
                buffer.delete_char_before(cursor.line(), cursor.col())
            {
                cursor.set_position(new_line, new_col);
            }
            true
        }
        "Delete" => {
            if cursor.has_selection() {
                delete_selection(buffer, cursor);
            } else {
                buffer.delete_char_at(cursor.line(), cursor.col());
            }
            true
        }
        "Enter" => {
            if cursor.has_selection() {
                delete_selection(buffer, cursor);
            }
            buffer.insert_newline(cursor.line(), cursor.col());
            cursor.set_position(cursor.line() + 1, 0);
            true
        }
        "Tab" => {
            if cursor.has_selection() {
                delete_selection(buffer, cursor);
            }
            // Insert 4 spaces instead of tab character
            buffer.insert(cursor.line(), cursor.col(), "    ");
            cursor.set_position(cursor.line(), cursor.col() + 4);
            true
        }

        // Keyboard shortcuts
        "a" | "A" if ctrl => {
            let last_line_len = buffer.line_len(line_count.saturating_sub(1));
            cursor.select_all(line_count, last_line_len);
            true
        }

        // Regular character input
        _ if key.len() == 1 && !ctrl => {
            let ch = key.chars().next().unwrap();
            if cursor.has_selection() {
                delete_selection(buffer, cursor);
            }
            buffer.insert_char(cursor.line(), cursor.col(), ch);
            cursor.set_position(cursor.line(), cursor.col() + 1);
            true
        }

        // Unhandled - let browser handle it (for copy/paste, etc.)
        _ => false,
    }
}

/// Delete the currently selected text
fn delete_selection(buffer: &mut TextBuffer, cursor: &mut Cursor) {
    if let Some(selection) = cursor.selection() {
        buffer.delete_range(
            selection.start_line,
            selection.start_col,
            selection.end_line,
            selection.end_col,
        );
        cursor.set_position(selection.start_line, selection.start_col);
        cursor.clear_selection();
    }
}
