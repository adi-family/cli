//! Font Atlas Generator
//!
//! Creates a texture atlas containing all ASCII glyphs for fast text rendering.
//! Uses Canvas 2D API to render glyphs, then uploads to WebGL texture.

use std::collections::HashMap;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, WebGlRenderingContext, WebGlTexture};

/// Information about a single glyph in the atlas
#[derive(Clone, Copy)]
pub struct GlyphInfo {
    /// Texture coordinates (u1, v1, u2, v2) normalized 0-1
    pub tex_coords: (f32, f32, f32, f32),
    /// Width of the glyph in pixels
    pub width: f32,
    /// Height of the glyph in pixels
    pub height: f32,
}

/// Font atlas containing all renderable glyphs
pub struct FontAtlas {
    texture: WebGlTexture,
    glyphs: HashMap<char, GlyphInfo>,
    atlas_size: u32,
    cell_width: u32,
    cell_height: u32,
}

impl FontAtlas {
    /// Create a new font atlas from the given canvas (used for 2D context)
    pub fn new(canvas: &HtmlCanvasElement, font_size: f32) -> Result<Self, JsValue> {
        // Calculate atlas dimensions
        // We need to fit ~95 printable ASCII characters (32-126)
        let chars_per_row = 16u32;
        let num_rows = 8u32;

        // Monospace fonts typically have width ~0.6 of height
        // Minimal padding to keep glyphs tight
        let padding = 1u32;
        let cell_width = (font_size * 0.55).ceil() as u32 + padding * 2;
        let cell_height = (font_size * 1.15).ceil() as u32 + padding * 2;

        // Make atlas size a power of 2 for better GPU performance
        let min_size = (chars_per_row * cell_width).max(num_rows * cell_height);
        let atlas_size = min_size.next_power_of_two();

        // Create an offscreen canvas for atlas generation
        let document = web_sys::window()
            .ok_or("No window")?
            .document()
            .ok_or("No document")?;

        let atlas_canvas: HtmlCanvasElement = document.create_element("canvas")?.dyn_into()?;

        atlas_canvas.set_width(atlas_size);
        atlas_canvas.set_height(atlas_size);

        let ctx: CanvasRenderingContext2d = atlas_canvas
            .get_context("2d")?
            .ok_or("Failed to get 2d context")?
            .dyn_into()?;

        // Clear with transparent background
        ctx.clear_rect(0.0, 0.0, atlas_size as f64, atlas_size as f64);

        // Setup high-quality font rendering
        // Use a specific monospace font for consistency (normal weight, not bold)
        let font = format!(
            "{}px 'SF Mono', 'Menlo', 'Monaco', 'Consolas', 'Liberation Mono', 'Courier New', monospace",
            font_size
        );
        ctx.set_font(&font);
        ctx.set_text_baseline("top");
        ctx.set_fill_style_str("#ffffff");

        // Enable font smoothing hints
        ctx.set_image_smoothing_enabled(true);

        // Generate glyph map
        let mut glyphs = HashMap::new();

        for i in 0..128u8 {
            let ch = i as char;
            if !ch.is_control() || ch == ' ' || ch == '\t' {
                let col = (i as u32 % chars_per_row) as u32;
                let row = (i as u32 / chars_per_row) as u32;

                let x = col * cell_width + padding;
                let y = row * cell_height + padding;

                // Render character
                let char_str = if ch == ' ' || ch == '\t' {
                    " ".to_string()
                } else {
                    ch.to_string()
                };

                ctx.fill_text(&char_str, x as f64, y as f64)?;

                // Calculate texture coordinates (include padding in the cell)
                let cell_x = col * cell_width;
                let cell_y = row * cell_height;
                let u1 = cell_x as f32 / atlas_size as f32;
                let v1 = cell_y as f32 / atlas_size as f32;
                let u2 = (cell_x + cell_width) as f32 / atlas_size as f32;
                let v2 = (cell_y + cell_height) as f32 / atlas_size as f32;

                glyphs.insert(
                    ch,
                    GlyphInfo {
                        tex_coords: (u1, v1, u2, v2),
                        width: cell_width as f32,
                        height: cell_height as f32,
                    },
                );
            }
        }

        // Add fallback for missing characters
        let fallback = glyphs.get(&'?').copied().unwrap_or(GlyphInfo {
            tex_coords: (0.0, 0.0, 0.1, 0.1),
            width: cell_width as f32,
            height: cell_height as f32,
        });

        // Create WebGL texture
        let gl = canvas
            .get_context("webgl")?
            .ok_or("Failed to get WebGL context")?
            .dyn_into::<WebGlRenderingContext>()?;

        let texture = gl.create_texture().ok_or("Failed to create texture")?;

        gl.bind_texture(WebGlRenderingContext::TEXTURE_2D, Some(&texture));

        // Set texture parameters - LINEAR for smooth high-DPI text
        gl.tex_parameteri(
            WebGlRenderingContext::TEXTURE_2D,
            WebGlRenderingContext::TEXTURE_WRAP_S,
            WebGlRenderingContext::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameteri(
            WebGlRenderingContext::TEXTURE_2D,
            WebGlRenderingContext::TEXTURE_WRAP_T,
            WebGlRenderingContext::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameteri(
            WebGlRenderingContext::TEXTURE_2D,
            WebGlRenderingContext::TEXTURE_MIN_FILTER,
            WebGlRenderingContext::LINEAR as i32,
        );
        gl.tex_parameteri(
            WebGlRenderingContext::TEXTURE_2D,
            WebGlRenderingContext::TEXTURE_MAG_FILTER,
            WebGlRenderingContext::LINEAR as i32,
        );

        // Upload atlas to texture
        gl.tex_image_2d_with_u32_and_u32_and_canvas(
            WebGlRenderingContext::TEXTURE_2D,
            0,
            WebGlRenderingContext::RGBA as i32,
            WebGlRenderingContext::RGBA,
            WebGlRenderingContext::UNSIGNED_BYTE,
            &atlas_canvas,
        )?;

        // Store fallback for unmapped characters
        for ch in ['\n', '\r', '\t'] {
            glyphs.entry(ch).or_insert(fallback);
        }

        Ok(FontAtlas {
            texture,
            glyphs,
            atlas_size,
            cell_width,
            cell_height,
        })
    }

    /// Get glyph information for a character
    pub fn get_glyph(&self, ch: char) -> Option<&GlyphInfo> {
        self.glyphs.get(&ch).or_else(|| self.glyphs.get(&'?'))
    }

    /// Get the WebGL texture
    pub fn texture(&self) -> Option<&WebGlTexture> {
        Some(&self.texture)
    }

    /// Get atlas dimensions
    pub fn atlas_size(&self) -> u32 {
        self.atlas_size
    }

    /// Get cell dimensions
    pub fn cell_size(&self) -> (u32, u32) {
        (self.cell_width, self.cell_height)
    }
}
