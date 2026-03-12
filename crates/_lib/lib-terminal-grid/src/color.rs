//! Terminal color types

/// Named terminal colors (standard 16-color palette)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NamedColor {
    #[default]
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl NamedColor {
    /// Create from ANSI color index (0-15)
    pub fn from_index(idx: u8) -> Option<Self> {
        match idx {
            0 => Some(NamedColor::Black),
            1 => Some(NamedColor::Red),
            2 => Some(NamedColor::Green),
            3 => Some(NamedColor::Yellow),
            4 => Some(NamedColor::Blue),
            5 => Some(NamedColor::Magenta),
            6 => Some(NamedColor::Cyan),
            7 => Some(NamedColor::White),
            8 => Some(NamedColor::BrightBlack),
            9 => Some(NamedColor::BrightRed),
            10 => Some(NamedColor::BrightGreen),
            11 => Some(NamedColor::BrightYellow),
            12 => Some(NamedColor::BrightBlue),
            13 => Some(NamedColor::BrightMagenta),
            14 => Some(NamedColor::BrightCyan),
            15 => Some(NamedColor::BrightWhite),
            _ => None,
        }
    }

    /// Get ANSI color index (0-15)
    pub fn to_index(self) -> u8 {
        match self {
            NamedColor::Black => 0,
            NamedColor::Red => 1,
            NamedColor::Green => 2,
            NamedColor::Yellow => 3,
            NamedColor::Blue => 4,
            NamedColor::Magenta => 5,
            NamedColor::Cyan => 6,
            NamedColor::White => 7,
            NamedColor::BrightBlack => 8,
            NamedColor::BrightRed => 9,
            NamedColor::BrightGreen => 10,
            NamedColor::BrightYellow => 11,
            NamedColor::BrightBlue => 12,
            NamedColor::BrightMagenta => 13,
            NamedColor::BrightCyan => 14,
            NamedColor::BrightWhite => 15,
        }
    }

    /// Get default RGB for this color (xterm defaults)
    pub fn default_rgb(self) -> (u8, u8, u8) {
        match self {
            NamedColor::Black => (0, 0, 0),
            NamedColor::Red => (205, 49, 49),
            NamedColor::Green => (13, 188, 121),
            NamedColor::Yellow => (229, 229, 16),
            NamedColor::Blue => (36, 114, 200),
            NamedColor::Magenta => (188, 63, 188),
            NamedColor::Cyan => (17, 168, 205),
            NamedColor::White => (229, 229, 229),
            NamedColor::BrightBlack => (102, 102, 102),
            NamedColor::BrightRed => (241, 76, 76),
            NamedColor::BrightGreen => (35, 209, 139),
            NamedColor::BrightYellow => (245, 245, 67),
            NamedColor::BrightBlue => (59, 142, 234),
            NamedColor::BrightMagenta => (214, 112, 214),
            NamedColor::BrightCyan => (41, 184, 219),
            NamedColor::BrightWhite => (255, 255, 255),
        }
    }
}

/// Terminal color (named, indexed 256, or true color RGB)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    /// Standard 16 ANSI colors
    Named(NamedColor),
    /// 256-color palette index
    Indexed(u8),
    /// True color RGB
    Rgb(u8, u8, u8),
    /// Default foreground/background (determined by theme)
    #[default]
    Default,
}

impl Color {
    /// Convert indexed color (0-255) to RGB
    pub fn index_to_rgb(idx: u8) -> (u8, u8, u8) {
        if idx < 16 {
            NamedColor::from_index(idx)
                .map(|c| c.default_rgb())
                .unwrap_or((255, 255, 255))
        } else if idx < 232 {
            // 216 color cube (6x6x6)
            let idx = idx - 16;
            let r = (idx / 36) % 6;
            let g = (idx / 6) % 6;
            let b = idx % 6;
            let to_val = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (to_val(r), to_val(g), to_val(b))
        } else {
            // 24 grayscale
            let gray = 8 + (idx - 232) * 10;
            (gray, gray, gray)
        }
    }

    /// Get RGB using default colors (for Default, uses white fg / black bg)
    pub fn to_rgb_default(self, is_foreground: bool) -> (u8, u8, u8) {
        match self {
            Color::Named(named) => named.default_rgb(),
            Color::Indexed(idx) => Self::index_to_rgb(idx),
            Color::Rgb(r, g, b) => (r, g, b),
            Color::Default => {
                if is_foreground {
                    (229, 229, 229) // Light gray
                } else {
                    (0, 0, 0) // Black
                }
            }
        }
    }
}
