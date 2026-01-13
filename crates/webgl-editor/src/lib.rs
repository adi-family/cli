//! Fast WebGL Text Editor
//!
//! A high-performance text editor that renders directly to WebGL,
//! bypassing the DOM for 60fps editing of large files.

mod buffer;
mod cursor;
mod font_atlas;
mod input;
mod renderer;
mod shaders;
mod syntax;
mod viewport;

use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, WebGlRenderingContext};

pub use buffer::TextBuffer;
pub use cursor::{Cursor, Selection};
pub use font_atlas::FontAtlas;
pub use renderer::Renderer;
pub use syntax::{Highlighter, Theme, TokenType};
pub use viewport::Viewport;

/// Initialize panic hook for better error messages in console
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Main editor instance exposed to JavaScript
#[wasm_bindgen]
pub struct Editor {
    renderer: Renderer,
    buffer: TextBuffer,
    cursor: Cursor,
    viewport: Viewport,
    highlighter: Highlighter,
    canvas_width: u32,
    canvas_height: u32,
    #[allow(dead_code)]
    font_size: f32,
    line_height: f32,
    char_width: f32,
    needs_redraw: bool,
}

#[wasm_bindgen]
impl Editor {
    /// Create a new editor instance attached to a canvas
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<Editor, JsValue> {
        let canvas_width = canvas.width();
        let canvas_height = canvas.height();

        let gl = canvas
            .get_context("webgl")?
            .ok_or("Failed to get WebGL context")?
            .dyn_into::<WebGlRenderingContext>()?;

        // Get device pixel ratio for HiDPI scaling (Mac Retina = 2.0)
        let dpr = web_sys::window()
            .map(|w| w.device_pixel_ratio() as f32)
            .unwrap_or(1.0)
            .max(2.0); // Ensure minimum 2x for crisp text

        // Base font size scaled by DPR for sharp rendering
        let base_font_size = 15.0;
        let font_size = base_font_size * dpr;

        let font_atlas = FontAtlas::new(&canvas, font_size)?;

        // Get actual cell size from font atlas to ensure perfect alignment
        let (cell_width, cell_height) = font_atlas.cell_size();
        let char_width = cell_width as f32;
        let line_height = cell_height as f32;

        let renderer = Renderer::new(gl, font_atlas)?;
        let highlighter = Highlighter::new();

        let visible_lines = (canvas_height as f32 / line_height).ceil() as usize;
        let visible_cols = (canvas_width as f32 / char_width).ceil() as usize;

        Ok(Editor {
            renderer,
            buffer: TextBuffer::new(),
            cursor: Cursor::new(),
            viewport: Viewport::new(visible_lines, visible_cols),
            highlighter,
            canvas_width,
            canvas_height,
            font_size,
            line_height,
            char_width,
            needs_redraw: true,
        })
    }

    /// Set the editor content
    #[wasm_bindgen]
    pub fn set_content(&mut self, text: &str) {
        self.buffer.set_content(text);
        self.cursor.reset();
        self.viewport.scroll_to(0, 0);
        self.needs_redraw = true;
    }

    /// Get the editor content
    #[wasm_bindgen]
    pub fn get_content(&self) -> String {
        self.buffer.get_content()
    }

    /// Render the editor
    #[wasm_bindgen]
    pub fn render(&mut self) {
        if !self.needs_redraw {
            return;
        }

        // Copy theme colors to avoid borrow issues
        let theme = self.highlighter.theme;

        self.renderer.clear_with_color(theme.background);
        self.renderer
            .set_viewport(self.canvas_width as i32, self.canvas_height as i32);

        // Render visible lines
        let start_line = self.viewport.scroll_line();
        let end_line = (start_line + self.viewport.visible_lines()).min(self.buffer.line_count());
        let line_num_width = 5.0 * self.char_width;

        for line_idx in start_line..end_line {
            if let Some(line) = self.buffer.get_line(line_idx) {
                let y = ((line_idx - start_line) as f32) * self.line_height;
                let scroll_offset = self.viewport.scroll_col() as f32 * self.char_width;

                // Render line number
                let line_num = format!("{:>4} ", line_idx + 1);
                self.renderer.render_text(
                    &line_num,
                    0.0,
                    y,
                    self.char_width,
                    self.line_height,
                    theme.line_number,
                );

                // Render line content with syntax highlighting
                let tokens = self.highlighter.highlight_line(&line);

                if tokens.is_empty() {
                    // No tokens, render as normal text
                    self.renderer.render_text(
                        &line,
                        line_num_width - scroll_offset,
                        y,
                        self.char_width,
                        self.line_height,
                        theme.normal,
                    );
                } else {
                    // Render each token with its color
                    let chars: Vec<char> = line.chars().collect();
                    let mut last_end = 0;

                    for token in &tokens {
                        // Render any gap before this token as normal
                        if token.start > last_end {
                            let gap_text: String = chars[last_end..token.start].iter().collect();
                            let gap_x = line_num_width + (last_end as f32 * self.char_width)
                                - scroll_offset;
                            self.renderer.render_text(
                                &gap_text,
                                gap_x,
                                y,
                                self.char_width,
                                self.line_height,
                                theme.normal,
                            );
                        }

                        // Render the token
                        let token_text: String = chars[token.start..token.end.min(chars.len())]
                            .iter()
                            .collect();
                        let token_x =
                            line_num_width + (token.start as f32 * self.char_width) - scroll_offset;
                        let color = theme.color_for(token.token_type);

                        self.renderer.render_text(
                            &token_text,
                            token_x,
                            y,
                            self.char_width,
                            self.line_height,
                            color,
                        );

                        last_end = token.end;
                    }

                    // Render any remaining text after last token
                    if last_end < chars.len() {
                        let remaining: String = chars[last_end..].iter().collect();
                        let remaining_x =
                            line_num_width + (last_end as f32 * self.char_width) - scroll_offset;
                        self.renderer.render_text(
                            &remaining,
                            remaining_x,
                            y,
                            self.char_width,
                            self.line_height,
                            theme.normal,
                        );
                    }
                }
            }
        }

        // Render selection if any (before cursor so cursor is on top)
        if let Some(selection) = self.cursor.selection() {
            self.render_selection(&selection, start_line, end_line, theme.selection);
        }

        // Render cursor
        let cursor_line = self.cursor.line();
        if cursor_line >= start_line && cursor_line < end_line {
            let cursor_x = line_num_width
                + (self.cursor.col() as f32 - self.viewport.scroll_col() as f32) * self.char_width;
            let cursor_y = (cursor_line - start_line) as f32 * self.line_height;

            self.renderer
                .render_cursor(cursor_x, cursor_y, 2.0, self.line_height, theme.cursor);
        }

        self.needs_redraw = false;
    }

    fn render_selection(
        &self,
        selection: &Selection,
        start_line: usize,
        end_line: usize,
        selection_color: [f32; 4],
    ) {
        let sel_start_line = selection.start_line.max(start_line);
        let sel_end_line = selection.end_line.min(end_line - 1);
        let line_num_width = 5.0 * self.char_width;

        for line_idx in sel_start_line..=sel_end_line {
            let line_len = self.buffer.line_len(line_idx);

            let start_col = if line_idx == selection.start_line {
                selection.start_col
            } else {
                0
            };

            let end_col = if line_idx == selection.end_line {
                selection.end_col
            } else {
                line_len
            };

            let x = line_num_width
                + (start_col as f32 - self.viewport.scroll_col() as f32) * self.char_width;
            let y = (line_idx - start_line) as f32 * self.line_height;
            let width = (end_col - start_col) as f32 * self.char_width;

            self.renderer
                .render_rect(x, y, width, self.line_height, selection_color);
        }
    }

    /// Handle keyboard input
    #[wasm_bindgen]
    pub fn handle_key(&mut self, key: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        let handled = input::handle_key(
            &mut self.buffer,
            &mut self.cursor,
            &mut self.viewport,
            key,
            ctrl,
            shift,
            alt,
        );

        if handled {
            self.ensure_cursor_visible();
            self.needs_redraw = true;
        }

        handled
    }

    /// Handle mouse click
    #[wasm_bindgen]
    pub fn handle_click(&mut self, x: f32, y: f32, shift: bool) {
        let line_num_width = 5.0 * self.char_width;

        if x < line_num_width {
            return; // Clicked on line numbers
        }

        let col = ((x - line_num_width) / self.char_width + self.viewport.scroll_col() as f32)
            .max(0.0) as usize;
        let line = (y / self.line_height) as usize + self.viewport.scroll_line();

        let line = line.min(self.buffer.line_count().saturating_sub(1));
        let col = col.min(self.buffer.line_len(line));

        if shift {
            self.cursor.select_to(line, col);
        } else {
            self.cursor.move_to(line, col);
        }

        self.needs_redraw = true;
    }

    /// Handle scroll event
    #[wasm_bindgen]
    pub fn handle_scroll(&mut self, delta_x: f32, delta_y: f32) {
        let lines = (delta_y / self.line_height) as i32;
        let cols = (delta_x / self.char_width) as i32;

        self.viewport
            .scroll_by(lines, cols, self.buffer.line_count());
        self.needs_redraw = true;
    }

    /// Resize the editor canvas
    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.canvas_width = width;
        self.canvas_height = height;
        self.viewport
            .set_visible_lines((height as f32 / self.line_height).ceil() as usize);
        self.viewport
            .set_visible_cols((width as f32 / self.char_width).ceil() as usize);
        self.needs_redraw = true;
    }

    /// Force a redraw
    #[wasm_bindgen]
    pub fn invalidate(&mut self) {
        self.needs_redraw = true;
    }

    /// Get cursor position for IME
    #[wasm_bindgen]
    pub fn get_cursor_position(&self) -> Vec<f32> {
        let x = 5.0 * self.char_width
            + (self.cursor.col() as f32 - self.viewport.scroll_col() as f32) * self.char_width;
        let y = (self.cursor.line() - self.viewport.scroll_line()) as f32 * self.line_height;
        vec![x, y]
    }

    /// Get line count
    #[wasm_bindgen]
    pub fn line_count(&self) -> usize {
        self.buffer.line_count()
    }

    /// Get current line number (1-indexed)
    #[wasm_bindgen]
    pub fn current_line(&self) -> usize {
        self.cursor.line() + 1
    }

    /// Get current column (1-indexed)
    #[wasm_bindgen]
    pub fn current_col(&self) -> usize {
        self.cursor.col() + 1
    }

    fn ensure_cursor_visible(&mut self) {
        self.viewport.ensure_visible(
            self.cursor.line(),
            self.cursor.col(),
            self.buffer.line_count(),
        );
    }
}
