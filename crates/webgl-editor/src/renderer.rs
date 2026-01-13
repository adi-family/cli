//! WebGL Renderer
//!
//! Handles all WebGL rendering operations including:
//! - Text rendering via textured quads
//! - Cursor and selection rendering via solid quads
//! - Batched draw calls for performance

use wasm_bindgen::prelude::*;
use web_sys::{WebGlBuffer, WebGlProgram, WebGlRenderingContext, WebGlUniformLocation};

use crate::font_atlas::FontAtlas;
use crate::shaders;

pub struct Renderer {
    gl: WebGlRenderingContext,
    font_atlas: FontAtlas,
    text_program: WebGlProgram,
    solid_program: WebGlProgram,
    text_position_buffer: WebGlBuffer,
    text_texcoord_buffer: WebGlBuffer,
    solid_position_buffer: WebGlBuffer,
    // Uniforms
    text_resolution_loc: WebGlUniformLocation,
    text_color_loc: WebGlUniformLocation,
    solid_resolution_loc: WebGlUniformLocation,
    solid_color_loc: WebGlUniformLocation,
    // Viewport
    width: i32,
    height: i32,
}

impl Renderer {
    pub fn new(gl: WebGlRenderingContext, font_atlas: FontAtlas) -> Result<Self, JsValue> {
        // Create text shader program
        let text_program = create_program(
            &gl,
            shaders::TEXT_VERTEX_SHADER,
            shaders::TEXT_FRAGMENT_SHADER,
        )?;

        // Create solid shader program
        let solid_program = create_program(
            &gl,
            shaders::SOLID_VERTEX_SHADER,
            shaders::SOLID_FRAGMENT_SHADER,
        )?;

        // Create buffers
        let text_position_buffer = gl.create_buffer().ok_or("Failed to create buffer")?;
        let text_texcoord_buffer = gl.create_buffer().ok_or("Failed to create buffer")?;
        let solid_position_buffer = gl.create_buffer().ok_or("Failed to create buffer")?;

        // Get uniform locations
        let text_resolution_loc = gl
            .get_uniform_location(&text_program, "u_resolution")
            .ok_or("Failed to get u_resolution location")?;
        let text_color_loc = gl
            .get_uniform_location(&text_program, "u_color")
            .ok_or("Failed to get u_color location")?;

        let solid_resolution_loc = gl
            .get_uniform_location(&solid_program, "u_resolution")
            .ok_or("Failed to get u_resolution location")?;
        let solid_color_loc = gl
            .get_uniform_location(&solid_program, "u_color")
            .ok_or("Failed to get u_color location")?;

        // Setup WebGL state
        gl.enable(WebGlRenderingContext::BLEND);
        gl.blend_func(
            WebGlRenderingContext::SRC_ALPHA,
            WebGlRenderingContext::ONE_MINUS_SRC_ALPHA,
        );

        Ok(Renderer {
            gl,
            font_atlas,
            text_program,
            solid_program,
            text_position_buffer,
            text_texcoord_buffer,
            solid_position_buffer,
            text_resolution_loc,
            text_color_loc,
            solid_resolution_loc,
            solid_color_loc,
            width: 800,
            height: 600,
        })
    }

    pub fn set_viewport(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
        self.gl.viewport(0, 0, width, height);
    }

    pub fn clear(&self) {
        self.gl.clear_color(0.12, 0.12, 0.14, 1.0); // Dark background
        self.gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);
    }

    pub fn clear_with_color(&self, color: [f32; 4]) {
        self.gl.clear_color(color[0], color[1], color[2], color[3]);
        self.gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);
    }

    /// Render a string of text at the given position
    pub fn render_text(
        &self,
        text: &str,
        x: f32,
        y: f32,
        char_width: f32,
        line_height: f32,
        color: [f32; 4],
    ) {
        if text.is_empty() {
            return;
        }

        let gl = &self.gl;

        // Use text shader program
        gl.use_program(Some(&self.text_program));

        // Set uniforms
        gl.uniform2f(
            Some(&self.text_resolution_loc),
            self.width as f32,
            self.height as f32,
        );
        gl.uniform4f(
            Some(&self.text_color_loc),
            color[0],
            color[1],
            color[2],
            color[3],
        );

        // Bind font texture
        gl.active_texture(WebGlRenderingContext::TEXTURE0);
        gl.bind_texture(WebGlRenderingContext::TEXTURE_2D, self.font_atlas.texture());

        // Build vertex data for all characters
        let mut positions: Vec<f32> = Vec::with_capacity(text.len() * 12);
        let mut texcoords: Vec<f32> = Vec::with_capacity(text.len() * 12);

        let mut curr_x = x;
        let glyph_height = line_height;

        for ch in text.chars() {
            if let Some(glyph) = self.font_atlas.get_glyph(ch) {
                let x1 = curr_x;
                let y1 = y;
                let x2 = curr_x + char_width;
                let y2 = y + glyph_height;

                // Two triangles for the quad
                // Triangle 1
                positions.extend_from_slice(&[x1, y1, x2, y1, x1, y2]);
                // Triangle 2
                positions.extend_from_slice(&[x1, y2, x2, y1, x2, y2]);

                // Texture coordinates
                let (u1, v1, u2, v2) = glyph.tex_coords;
                texcoords.extend_from_slice(&[u1, v1, u2, v1, u1, v2]);
                texcoords.extend_from_slice(&[u1, v2, u2, v1, u2, v2]);
            }

            curr_x += char_width;
        }

        if positions.is_empty() {
            return;
        }

        // Upload position data
        gl.bind_buffer(
            WebGlRenderingContext::ARRAY_BUFFER,
            Some(&self.text_position_buffer),
        );
        unsafe {
            let positions_array = js_sys::Float32Array::view(&positions);
            gl.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &positions_array,
                WebGlRenderingContext::DYNAMIC_DRAW,
            );
        }

        // Setup position attribute
        let position_loc = gl.get_attrib_location(&self.text_program, "a_position") as u32;
        gl.enable_vertex_attrib_array(position_loc);
        gl.vertex_attrib_pointer_with_i32(
            position_loc,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            0,
            0,
        );

        // Upload texcoord data
        gl.bind_buffer(
            WebGlRenderingContext::ARRAY_BUFFER,
            Some(&self.text_texcoord_buffer),
        );
        unsafe {
            let texcoords_array = js_sys::Float32Array::view(&texcoords);
            gl.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &texcoords_array,
                WebGlRenderingContext::DYNAMIC_DRAW,
            );
        }

        // Setup texcoord attribute
        let texcoord_loc = gl.get_attrib_location(&self.text_program, "a_texcoord") as u32;
        gl.enable_vertex_attrib_array(texcoord_loc);
        gl.vertex_attrib_pointer_with_i32(
            texcoord_loc,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            0,
            0,
        );

        // Draw
        let vertex_count = (positions.len() / 2) as i32;
        gl.draw_arrays(WebGlRenderingContext::TRIANGLES, 0, vertex_count);
    }

    /// Render a solid color rectangle (for cursor, selection, etc.)
    pub fn render_rect(&self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        let gl = &self.gl;

        gl.use_program(Some(&self.solid_program));

        gl.uniform2f(
            Some(&self.solid_resolution_loc),
            self.width as f32,
            self.height as f32,
        );
        gl.uniform4f(
            Some(&self.solid_color_loc),
            color[0],
            color[1],
            color[2],
            color[3],
        );

        let x1 = x;
        let y1 = y;
        let x2 = x + width;
        let y2 = y + height;

        let positions: [f32; 12] = [
            x1, y1, x2, y1, x1, y2, // Triangle 1
            x1, y2, x2, y1, x2, y2, // Triangle 2
        ];

        gl.bind_buffer(
            WebGlRenderingContext::ARRAY_BUFFER,
            Some(&self.solid_position_buffer),
        );
        unsafe {
            let positions_array = js_sys::Float32Array::view(&positions);
            gl.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &positions_array,
                WebGlRenderingContext::DYNAMIC_DRAW,
            );
        }

        let position_loc = gl.get_attrib_location(&self.solid_program, "a_position") as u32;
        gl.enable_vertex_attrib_array(position_loc);
        gl.vertex_attrib_pointer_with_i32(
            position_loc,
            2,
            WebGlRenderingContext::FLOAT,
            false,
            0,
            0,
        );

        gl.draw_arrays(WebGlRenderingContext::TRIANGLES, 0, 6);
    }

    /// Render cursor (thin vertical bar)
    pub fn render_cursor(&self, x: f32, y: f32, width: f32, height: f32, color: [f32; 4]) {
        self.render_rect(x, y, width, height, color);
    }
}

/// Compile a shader
fn compile_shader(
    gl: &WebGlRenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<web_sys::WebGlShader, JsValue> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or("Failed to create shader")?;

    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    let success = gl
        .get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false);

    if success {
        Ok(shader)
    } else {
        let log = gl.get_shader_info_log(&shader).unwrap_or_default();
        gl.delete_shader(Some(&shader));
        Err(JsValue::from_str(&format!(
            "Shader compilation failed: {}",
            log
        )))
    }
}

/// Create a shader program
fn create_program(
    gl: &WebGlRenderingContext,
    vertex_source: &str,
    fragment_source: &str,
) -> Result<WebGlProgram, JsValue> {
    let vertex_shader = compile_shader(gl, WebGlRenderingContext::VERTEX_SHADER, vertex_source)?;
    let fragment_shader =
        compile_shader(gl, WebGlRenderingContext::FRAGMENT_SHADER, fragment_source)?;

    let program = gl.create_program().ok_or("Failed to create program")?;

    gl.attach_shader(&program, &vertex_shader);
    gl.attach_shader(&program, &fragment_shader);
    gl.link_program(&program);

    let success = gl
        .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false);

    // Clean up shaders (they're now part of the program)
    gl.delete_shader(Some(&vertex_shader));
    gl.delete_shader(Some(&fragment_shader));

    if success {
        Ok(program)
    } else {
        let log = gl.get_program_info_log(&program).unwrap_or_default();
        gl.delete_program(Some(&program));
        Err(JsValue::from_str(&format!(
            "Program linking failed: {}",
            log
        )))
    }
}
