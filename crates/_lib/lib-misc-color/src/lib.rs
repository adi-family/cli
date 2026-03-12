//! Unified color type with lazy conversion
//!
//! Colors are stored in their original format and only converted when needed.

/// Unified color representation supporting multiple formats.
/// Conversion happens lazily when accessing a specific format.
#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    /// RGB as bytes [0-255]
    Rgb(u8, u8, u8),
    /// RGBA as bytes [0-255]
    Rgba(u8, u8, u8, u8),
    /// RGB as normalized floats [0.0-1.0]
    RgbFloat(f32, f32, f32),
    /// RGBA as normalized floats [0.0-1.0]
    RgbaFloat(f32, f32, f32, f32),
    /// Hex color string (with or without #)
    Hex(String),
}

impl Color {
    // ─────────────────────────────────────────────────────────────────────────
    // Constructors
    // ─────────────────────────────────────────────────────────────────────────

    /// Create from RGB bytes
    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb(r, g, b)
    }

    /// Create from RGBA bytes
    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::Rgba(r, g, b, a)
    }

    /// Create from RGB floats [0.0-1.0]
    #[inline]
    pub const fn rgb_float(r: f32, g: f32, b: f32) -> Self {
        Self::RgbFloat(r, g, b)
    }

    /// Create from RGBA floats [0.0-1.0]
    #[inline]
    pub const fn rgba_float(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::RgbaFloat(r, g, b, a)
    }

    /// Create from hex string (with or without #)
    #[inline]
    pub fn hex(s: impl Into<String>) -> Self {
        Self::Hex(s.into())
    }

    /// Parse hex string, returning None for invalid format
    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        match s.len() {
            // RGB shorthand: #RGB -> #RRGGBB
            3 => {
                let r = u8::from_str_radix(&s[0..1], 16).ok()?;
                let g = u8::from_str_radix(&s[1..2], 16).ok()?;
                let b = u8::from_str_radix(&s[2..3], 16).ok()?;
                Some(Self::Rgb(r * 17, g * 17, b * 17))
            }
            // RGBA shorthand: #RGBA -> #RRGGBBAA
            4 => {
                let r = u8::from_str_radix(&s[0..1], 16).ok()?;
                let g = u8::from_str_radix(&s[1..2], 16).ok()?;
                let b = u8::from_str_radix(&s[2..3], 16).ok()?;
                let a = u8::from_str_radix(&s[3..4], 16).ok()?;
                Some(Self::Rgba(r * 17, g * 17, b * 17, a * 17))
            }
            // Full RGB: #RRGGBB
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                Some(Self::Rgb(r, g, b))
            }
            // Full RGBA: #RRGGBBAA
            8 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                let a = u8::from_str_radix(&s[6..8], 16).ok()?;
                Some(Self::Rgba(r, g, b, a))
            }
            _ => None,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Lazy accessors (convert on read)
    // ─────────────────────────────────────────────────────────────────────────

    /// Get as RGB bytes tuple
    pub fn as_rgb(&self) -> (u8, u8, u8) {
        match self {
            Self::Rgb(r, g, b) => (*r, *g, *b),
            Self::Rgba(r, g, b, _) => (*r, *g, *b),
            Self::RgbFloat(r, g, b) => (
                (r.clamp(0.0, 1.0) * 255.0) as u8,
                (g.clamp(0.0, 1.0) * 255.0) as u8,
                (b.clamp(0.0, 1.0) * 255.0) as u8,
            ),
            Self::RgbaFloat(r, g, b, _) => (
                (r.clamp(0.0, 1.0) * 255.0) as u8,
                (g.clamp(0.0, 1.0) * 255.0) as u8,
                (b.clamp(0.0, 1.0) * 255.0) as u8,
            ),
            Self::Hex(s) => Self::from_hex(s).map(|c| c.as_rgb()).unwrap_or((0, 0, 0)),
        }
    }

    /// Get as RGBA bytes tuple
    pub fn as_rgba(&self) -> (u8, u8, u8, u8) {
        match self {
            Self::Rgb(r, g, b) => (*r, *g, *b, 255),
            Self::Rgba(r, g, b, a) => (*r, *g, *b, *a),
            Self::RgbFloat(r, g, b) => (
                (r.clamp(0.0, 1.0) * 255.0) as u8,
                (g.clamp(0.0, 1.0) * 255.0) as u8,
                (b.clamp(0.0, 1.0) * 255.0) as u8,
                255,
            ),
            Self::RgbaFloat(r, g, b, a) => (
                (r.clamp(0.0, 1.0) * 255.0) as u8,
                (g.clamp(0.0, 1.0) * 255.0) as u8,
                (b.clamp(0.0, 1.0) * 255.0) as u8,
                (a.clamp(0.0, 1.0) * 255.0) as u8,
            ),
            Self::Hex(s) => Self::from_hex(s)
                .map(|c| c.as_rgba())
                .unwrap_or((0, 0, 0, 255)),
        }
    }

    /// Get as RGB float array [0.0-1.0]
    pub fn as_rgb_float(&self) -> [f32; 3] {
        match self {
            Self::Rgb(r, g, b) => [*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0],
            Self::Rgba(r, g, b, _) => [*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0],
            Self::RgbFloat(r, g, b) => [*r, *g, *b],
            Self::RgbaFloat(r, g, b, _) => [*r, *g, *b],
            Self::Hex(s) => Self::from_hex(s)
                .map(|c| c.as_rgb_float())
                .unwrap_or([0.0, 0.0, 0.0]),
        }
    }

    /// Get as RGBA float array [0.0-1.0]
    pub fn as_rgba_float(&self) -> [f32; 4] {
        match self {
            Self::Rgb(r, g, b) => [*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0, 1.0],
            Self::Rgba(r, g, b, a) => [
                *r as f32 / 255.0,
                *g as f32 / 255.0,
                *b as f32 / 255.0,
                *a as f32 / 255.0,
            ],
            Self::RgbFloat(r, g, b) => [*r, *g, *b, 1.0],
            Self::RgbaFloat(r, g, b, a) => [*r, *g, *b, *a],
            Self::Hex(s) => Self::from_hex(s)
                .map(|c| c.as_rgba_float())
                .unwrap_or([0.0, 0.0, 0.0, 1.0]),
        }
    }

    /// Get as hex string (always returns #RRGGBB or #RRGGBBAA)
    pub fn as_hex(&self) -> String {
        match self {
            Self::Hex(s) => {
                let s = s.strip_prefix('#').unwrap_or(s);
                format!("#{s}")
            }
            _ => {
                let (r, g, b, a) = self.as_rgba();
                if a == 255 {
                    format!("#{r:02x}{g:02x}{b:02x}")
                } else {
                    format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
                }
            }
        }
    }

    /// Get alpha value as float [0.0-1.0]
    pub fn alpha(&self) -> f32 {
        match self {
            Self::Rgb(_, _, _) | Self::RgbFloat(_, _, _) => 1.0,
            Self::Rgba(_, _, _, a) => *a as f32 / 255.0,
            Self::RgbaFloat(_, _, _, a) => *a,
            Self::Hex(s) => Self::from_hex(s).map(|c| c.alpha()).unwrap_or(1.0),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Transformations (return new Color)
    // ─────────────────────────────────────────────────────────────────────────

    /// Create new color with specified alpha
    pub fn with_alpha(&self, alpha: f32) -> Self {
        let [r, g, b, _] = self.as_rgba_float();
        Self::RgbaFloat(r, g, b, alpha.clamp(0.0, 1.0))
    }

    /// Lighten color by amount [0.0-1.0]
    pub fn lighten(&self, amount: f32) -> Self {
        let [r, g, b, a] = self.as_rgba_float();
        Self::RgbaFloat(
            (r + amount).min(1.0),
            (g + amount).min(1.0),
            (b + amount).min(1.0),
            a,
        )
    }

    /// Darken color by amount [0.0-1.0]
    pub fn darken(&self, amount: f32) -> Self {
        let [r, g, b, a] = self.as_rgba_float();
        Self::RgbaFloat(
            (r - amount).max(0.0),
            (g - amount).max(0.0),
            (b - amount).max(0.0),
            a,
        )
    }

    /// Mix with another color (0.0 = self, 1.0 = other)
    pub fn mix(&self, other: &Self, t: f32) -> Self {
        let [r1, g1, b1, a1] = self.as_rgba_float();
        let [r2, g2, b2, a2] = other.as_rgba_float();
        let t = t.clamp(0.0, 1.0);
        Self::RgbaFloat(
            r1 + (r2 - r1) * t,
            g1 + (g2 - g1) * t,
            b1 + (b2 - b1) * t,
            a1 + (a2 - a1) * t,
        )
    }

    /// Invert color (RGB only, alpha preserved)
    pub fn invert(&self) -> Self {
        let [r, g, b, a] = self.as_rgba_float();
        Self::RgbaFloat(1.0 - r, 1.0 - g, 1.0 - b, a)
    }

    /// Convert to grayscale using luminance formula
    pub fn grayscale(&self) -> Self {
        let [r, g, b, a] = self.as_rgba_float();
        let lum = 0.299 * r + 0.587 * g + 0.114 * b;
        Self::RgbaFloat(lum, lum, lum, a)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// From implementations for ergonomic construction
// ─────────────────────────────────────────────────────────────────────────────

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::Rgb(r, g, b)
    }
}

impl From<(u8, u8, u8, u8)> for Color {
    fn from((r, g, b, a): (u8, u8, u8, u8)) -> Self {
        Self::Rgba(r, g, b, a)
    }
}

impl From<[u8; 3]> for Color {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self::Rgb(r, g, b)
    }
}

impl From<[u8; 4]> for Color {
    fn from([r, g, b, a]: [u8; 4]) -> Self {
        Self::Rgba(r, g, b, a)
    }
}

impl From<[f32; 3]> for Color {
    fn from([r, g, b]: [f32; 3]) -> Self {
        Self::RgbFloat(r, g, b)
    }
}

impl From<[f32; 4]> for Color {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Self::RgbaFloat(r, g, b, a)
    }
}

impl From<&str> for Color {
    fn from(s: &str) -> Self {
        Self::from_hex(s).unwrap_or(Self::Hex(s.to_string()))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Default
// ─────────────────────────────────────────────────────────────────────────────

impl Default for Color {
    fn default() -> Self {
        Self::Rgb(0, 0, 0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_constructor() {
        let c = Color::rgb(255, 128, 0);
        assert_eq!(c.as_rgb(), (255, 128, 0));
    }

    #[test]
    fn test_rgba_constructor() {
        let c = Color::rgba(255, 128, 0, 128);
        assert_eq!(c.as_rgba(), (255, 128, 0, 128));
    }

    #[test]
    fn test_rgb_to_rgba() {
        let c = Color::rgb(255, 128, 0);
        assert_eq!(c.as_rgba(), (255, 128, 0, 255));
    }

    #[test]
    fn test_float_to_bytes() {
        let c = Color::rgba_float(1.0, 0.5, 0.0, 1.0);
        let (r, g, b, a) = c.as_rgba();
        assert_eq!(r, 255);
        assert!((g as i32 - 127).abs() <= 1); // Allow rounding
        assert_eq!(b, 0);
        assert_eq!(a, 255);
    }

    #[test]
    fn test_bytes_to_float() {
        let c = Color::rgb(255, 0, 128);
        let [r, g, b] = c.as_rgb_float();
        assert!((r - 1.0).abs() < 0.01);
        assert!((g - 0.0).abs() < 0.01);
        assert!((b - 0.502).abs() < 0.01);
    }

    #[test]
    fn test_hex_parsing_6_digit() {
        let c = Color::from_hex("#ff8000").unwrap();
        assert_eq!(c.as_rgb(), (255, 128, 0));
    }

    #[test]
    fn test_hex_parsing_8_digit() {
        let c = Color::from_hex("#ff800080").unwrap();
        assert_eq!(c.as_rgba(), (255, 128, 0, 128));
    }

    #[test]
    fn test_hex_parsing_3_digit() {
        let c = Color::from_hex("#f80").unwrap();
        assert_eq!(c.as_rgb(), (255, 136, 0));
    }

    #[test]
    fn test_hex_parsing_without_hash() {
        let c = Color::from_hex("ff8000").unwrap();
        assert_eq!(c.as_rgb(), (255, 128, 0));
    }

    #[test]
    fn test_as_hex() {
        let c = Color::rgb(255, 128, 0);
        assert_eq!(c.as_hex(), "#ff8000");
    }

    #[test]
    fn test_as_hex_with_alpha() {
        let c = Color::rgba(255, 128, 0, 128);
        assert_eq!(c.as_hex(), "#ff800080");
    }

    #[test]
    fn test_with_alpha() {
        let c = Color::rgb(255, 128, 0).with_alpha(0.5);
        assert!((c.alpha() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_lighten() {
        let c = Color::rgb(128, 128, 128).lighten(0.1);
        let [r, g, b, _] = c.as_rgba_float();
        assert!(r > 0.5);
        assert!(g > 0.5);
        assert!(b > 0.5);
    }

    #[test]
    fn test_darken() {
        let c = Color::rgb(128, 128, 128).darken(0.1);
        let [r, g, b, _] = c.as_rgba_float();
        assert!(r < 0.5);
        assert!(g < 0.5);
        assert!(b < 0.5);
    }

    #[test]
    fn test_mix() {
        let white = Color::rgb(255, 255, 255);
        let black = Color::rgb(0, 0, 0);
        let gray = white.mix(&black, 0.5);
        let (r, g, b, _) = gray.as_rgba();
        assert!((r as i32 - 127).abs() <= 1);
        assert!((g as i32 - 127).abs() <= 1);
        assert!((b as i32 - 127).abs() <= 1);
    }

    #[test]
    fn test_invert() {
        let c = Color::rgb(255, 0, 128).invert();
        let (r, g, b, _) = c.as_rgba();
        assert_eq!(r, 0);
        assert_eq!(g, 255);
        assert!((b as i32 - 127).abs() <= 1);
    }

    #[test]
    fn test_grayscale() {
        let c = Color::rgb(255, 0, 0).grayscale();
        let [r, g, b, _] = c.as_rgba_float();
        assert!((r - g).abs() < 0.01);
        assert!((g - b).abs() < 0.01);
    }

    #[test]
    fn test_from_tuple() {
        let c: Color = (255, 128, 0).into();
        assert_eq!(c.as_rgb(), (255, 128, 0));
    }

    #[test]
    fn test_from_array() {
        let c: Color = [0.5f32, 0.5, 0.5, 1.0].into();
        assert!(matches!(c, Color::RgbaFloat(_, _, _, _)));
    }

    #[test]
    fn test_from_str() {
        let c: Color = "#ff8000".into();
        assert_eq!(c.as_rgb(), (255, 128, 0));
    }

    #[test]
    fn test_alpha_rgb() {
        let c = Color::rgb(255, 128, 0);
        assert!((c.alpha() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_alpha_rgba() {
        let c = Color::rgba(255, 128, 0, 128);
        assert!((c.alpha() - 0.502).abs() < 0.01);
    }

    #[test]
    fn test_clamp_float_values() {
        let c = Color::rgba_float(1.5, -0.5, 0.5, 2.0);
        let (r, g, b, a) = c.as_rgba();
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 127);
        assert_eq!(a, 255);
    }
}
