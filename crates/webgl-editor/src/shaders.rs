//! WebGL Shaders for text rendering
//!
//! Uses textured quads for glyph rendering and solid color quads for
//! cursor/selection highlighting.

/// Vertex shader for textured quads (glyphs)
pub const TEXT_VERTEX_SHADER: &str = r#"
    attribute vec2 a_position;
    attribute vec2 a_texcoord;

    uniform vec2 u_resolution;

    varying vec2 v_texcoord;

    void main() {
        // Convert from pixels to clip space (-1 to 1)
        vec2 clipSpace = (a_position / u_resolution) * 2.0 - 1.0;
        // Flip Y axis (canvas Y goes down, WebGL Y goes up)
        gl_Position = vec4(clipSpace.x, -clipSpace.y, 0.0, 1.0);
        v_texcoord = a_texcoord;
    }
"#;

/// Fragment shader for textured quads (glyphs)
pub const TEXT_FRAGMENT_SHADER: &str = r#"
    precision mediump float;

    uniform sampler2D u_texture;
    uniform vec4 u_color;

    varying vec2 v_texcoord;

    void main() {
        float alpha = texture2D(u_texture, v_texcoord).a;
        gl_FragColor = vec4(u_color.rgb, u_color.a * alpha);
    }
"#;

/// Vertex shader for solid color quads (cursor, selection, backgrounds)
pub const SOLID_VERTEX_SHADER: &str = r#"
    attribute vec2 a_position;

    uniform vec2 u_resolution;

    void main() {
        vec2 clipSpace = (a_position / u_resolution) * 2.0 - 1.0;
        gl_Position = vec4(clipSpace.x, -clipSpace.y, 0.0, 1.0);
    }
"#;

/// Fragment shader for solid color quads
pub const SOLID_FRAGMENT_SHADER: &str = r#"
    precision mediump float;

    uniform vec4 u_color;

    void main() {
        gl_FragColor = u_color;
    }
"#;
