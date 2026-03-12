//! Built-in terminal themes

use crate::animation::AnimationConfig;
use crate::layout::LayoutConfig;
use crate::overlay::OverlayTheme;
use crate::palette::{ColorPalette, CursorConfig, CursorStyle, SidebarTheme};
use crate::typography::Typography;
use lib_misc_color::Color;

/// Complete terminal theme
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub background: Color,
    pub foreground: Color,
    pub selection: Color,
    pub cursor: CursorConfig,
    pub palette: ColorPalette,
    pub sidebar: SidebarTheme,
    pub typography: Typography,
    pub layout: LayoutConfig,
    pub overlay: OverlayTheme,
    pub animation: AnimationConfig,
}

/// Raw theme color data - minimal struct for defining a theme
struct ThemeColors {
    name: &'static str,
    background: [f32; 4],
    foreground: (u8, u8, u8),
    selection: [f32; 4],
    cursor_color: [f32; 4],
    normal_colors: [(u8, u8, u8); 8],
    bright_colors: [(u8, u8, u8); 8],
    accent: [f32; 4],
    muted: [f32; 4],
    border_radius: f32,
    padding: f32,
}

impl ThemeColors {
    fn build(&self) -> Theme {
        let bg = Color::from(self.background);
        let fg = Color::from(self.foreground);
        let selection = Color::from(self.selection);
        let accent = Color::from(self.accent);
        let muted = Color::from(self.muted);

        let header_bg = bg.lighten(0.03);
        let hover_bg = bg.lighten(0.06);
        let active_bg = bg.lighten(0.10);

        // Create layout with theme-specific border radius and padding
        let layout = LayoutConfig {
            border_radius: self.border_radius,
            content_padding: self.padding,
            ..Default::default()
        };

        Theme {
            name: self.name,
            background: bg.clone(),
            foreground: fg.clone(),
            selection: selection.clone(),
            cursor: CursorConfig {
                style: CursorStyle::Block,
                color: Color::from(self.cursor_color),
                blink: true,
                blink_rate: 1.2,
            },
            palette: ColorPalette {
                black: Color::from(self.normal_colors[0]),
                red: Color::from(self.normal_colors[1]),
                green: Color::from(self.normal_colors[2]),
                yellow: Color::from(self.normal_colors[3]),
                blue: Color::from(self.normal_colors[4]),
                magenta: Color::from(self.normal_colors[5]),
                cyan: Color::from(self.normal_colors[6]),
                white: Color::from(self.normal_colors[7]),
                bright_black: Color::from(self.bright_colors[0]),
                bright_red: Color::from(self.bright_colors[1]),
                bright_green: Color::from(self.bright_colors[2]),
                bright_yellow: Color::from(self.bright_colors[3]),
                bright_blue: Color::from(self.bright_colors[4]),
                bright_magenta: Color::from(self.bright_colors[5]),
                bright_cyan: Color::from(self.bright_colors[6]),
                bright_white: Color::from(self.bright_colors[7]),
            },
            sidebar: SidebarTheme {
                background: bg.clone(),
                header_background: header_bg.clone(),
                session_background: bg.clone(),
                session_hover: hover_bg,
                session_active: active_bg,
                text: fg,
                text_dim: muted,
                text_active: accent.clone(),
                border: selection.clone(),
                new_button: accent.clone(),
                new_button_hover: accent.lighten(0.1),
                new_button_text: bg,
                close_button: Color::from(self.normal_colors[1]),
                close_button_hover: Color::from(self.bright_colors[1]),
                scrollbar: header_bg,
                scrollbar_thumb: selection.lighten(0.1),
            },
            typography: Typography::default(),
            layout,
            overlay: OverlayTheme::default(),
            animation: AnimationConfig::default(),
        }
    }
}

// Theme definitions
const TOKYO_NIGHT: ThemeColors = ThemeColors {
    name: "Tokyo Night",
    background: [0.10, 0.11, 0.15, 1.0],
    foreground: (169, 177, 214),
    selection: [0.24, 0.28, 0.47, 0.5],
    cursor_color: [0.78, 0.86, 0.98, 0.9],
    normal_colors: [
        (21, 22, 30),
        (247, 118, 142),
        (158, 206, 106),
        (224, 175, 104),
        (122, 162, 247),
        (187, 154, 247),
        (125, 207, 255),
        (169, 177, 214),
    ],
    bright_colors: [
        (65, 72, 104),
        (247, 118, 142),
        (158, 206, 106),
        (224, 175, 104),
        (122, 162, 247),
        (187, 154, 247),
        (125, 207, 255),
        (192, 202, 245),
    ],
    accent: [0.48, 0.64, 0.97, 1.0],
    muted: [0.40, 0.42, 0.54, 1.0],
    border_radius: 6.0,
    padding: 12.0,
};

const DRACULA: ThemeColors = ThemeColors {
    name: "Dracula",
    background: [0.16, 0.16, 0.21, 1.0],
    foreground: (248, 248, 242),
    selection: [0.27, 0.28, 0.35, 0.6],
    cursor_color: [0.97, 0.97, 0.95, 0.9],
    normal_colors: [
        (33, 34, 44),
        (255, 85, 85),
        (80, 250, 123),
        (241, 250, 140),
        (98, 114, 164),
        (255, 121, 198),
        (139, 233, 253),
        (248, 248, 242),
    ],
    bright_colors: [
        (98, 114, 164),
        (255, 110, 110),
        (105, 255, 148),
        (255, 255, 165),
        (125, 139, 189),
        (255, 146, 223),
        (164, 255, 255),
        (255, 255, 255),
    ],
    accent: [0.31, 0.98, 0.48, 1.0],
    muted: [0.55, 0.57, 0.68, 1.0],
    border_radius: 6.0,
    padding: 12.0,
};

const ONE_DARK: ThemeColors = ThemeColors {
    name: "One Dark",
    background: [0.16, 0.17, 0.20, 1.0],
    foreground: (171, 178, 191),
    selection: [0.24, 0.26, 0.32, 0.5],
    cursor_color: [0.53, 0.75, 0.98, 0.9],
    normal_colors: [
        (40, 44, 52),
        (224, 108, 117),
        (152, 195, 121),
        (229, 192, 123),
        (97, 175, 239),
        (198, 120, 221),
        (86, 182, 194),
        (171, 178, 191),
    ],
    bright_colors: [
        (92, 99, 112),
        (224, 108, 117),
        (152, 195, 121),
        (229, 192, 123),
        (97, 175, 239),
        (198, 120, 221),
        (86, 182, 194),
        (255, 255, 255),
    ],
    accent: [0.38, 0.69, 0.94, 1.0],
    muted: [0.45, 0.48, 0.53, 1.0],
    border_radius: 4.0,
    padding: 10.0,
};

const CATPPUCCIN_MOCHA: ThemeColors = ThemeColors {
    name: "Catppuccin Mocha",
    background: [0.12, 0.12, 0.18, 1.0],
    foreground: (205, 214, 244),
    selection: [0.27, 0.29, 0.42, 0.5],
    cursor_color: [0.95, 0.55, 0.66, 0.9],
    normal_colors: [
        (69, 71, 90),
        (243, 139, 168),
        (166, 227, 161),
        (249, 226, 175),
        (137, 180, 250),
        (203, 166, 247),
        (148, 226, 213),
        (186, 194, 222),
    ],
    bright_colors: [
        (88, 91, 112),
        (243, 139, 168),
        (166, 227, 161),
        (249, 226, 175),
        (137, 180, 250),
        (203, 166, 247),
        (148, 226, 213),
        (165, 173, 206),
    ],
    accent: [0.54, 0.71, 0.98, 1.0],
    muted: [0.50, 0.53, 0.66, 1.0],
    border_radius: 8.0,
    padding: 12.0,
};

const GRUVBOX_DARK: ThemeColors = ThemeColors {
    name: "Gruvbox Dark",
    background: [0.16, 0.15, 0.13, 1.0],
    foreground: (235, 219, 178),
    selection: [0.33, 0.32, 0.27, 0.5],
    cursor_color: [0.92, 0.86, 0.70, 0.9],
    normal_colors: [
        (40, 40, 40),
        (204, 36, 29),
        (152, 151, 26),
        (215, 153, 33),
        (69, 133, 136),
        (177, 98, 134),
        (104, 157, 106),
        (168, 153, 132),
    ],
    bright_colors: [
        (146, 131, 116),
        (251, 73, 52),
        (184, 187, 38),
        (250, 189, 47),
        (131, 165, 152),
        (211, 134, 155),
        (142, 192, 124),
        (235, 219, 178),
    ],
    accent: [0.98, 0.74, 0.18, 1.0],
    muted: [0.66, 0.60, 0.52, 1.0],
    border_radius: 4.0,
    padding: 10.0,
};

const NORD: ThemeColors = ThemeColors {
    name: "Nord",
    background: [0.18, 0.20, 0.25, 1.0],
    foreground: (216, 222, 233),
    selection: [0.26, 0.30, 0.37, 0.5],
    cursor_color: [0.85, 0.87, 0.91, 0.9],
    normal_colors: [
        (59, 66, 82),
        (191, 97, 106),
        (163, 190, 140),
        (235, 203, 139),
        (129, 161, 193),
        (180, 142, 173),
        (136, 192, 208),
        (229, 233, 240),
    ],
    bright_colors: [
        (76, 86, 106),
        (191, 97, 106),
        (163, 190, 140),
        (235, 203, 139),
        (129, 161, 193),
        (180, 142, 173),
        (143, 188, 187),
        (236, 239, 244),
    ],
    accent: [0.51, 0.63, 0.76, 1.0],
    muted: [0.55, 0.60, 0.70, 1.0],
    border_radius: 6.0,
    padding: 12.0,
};

const SOLARIZED_DARK: ThemeColors = ThemeColors {
    name: "Solarized Dark",
    background: [0.00, 0.17, 0.21, 1.0],
    foreground: (131, 148, 150),
    selection: [0.03, 0.21, 0.26, 0.5],
    cursor_color: [0.51, 0.58, 0.59, 0.9],
    normal_colors: [
        (7, 54, 66),
        (220, 50, 47),
        (133, 153, 0),
        (181, 137, 0),
        (38, 139, 210),
        (211, 54, 130),
        (42, 161, 152),
        (238, 232, 213),
    ],
    bright_colors: [
        (0, 43, 54),
        (203, 75, 22),
        (88, 110, 117),
        (101, 123, 131),
        (131, 148, 150),
        (108, 113, 196),
        (147, 161, 161),
        (253, 246, 227),
    ],
    accent: [0.15, 0.55, 0.82, 1.0],
    muted: [0.35, 0.43, 0.46, 1.0],
    border_radius: 4.0,
    padding: 10.0,
};

const MONOKAI_PRO: ThemeColors = ThemeColors {
    name: "Monokai Pro",
    background: [0.16, 0.16, 0.15, 1.0],
    foreground: (252, 252, 250),
    selection: [0.28, 0.27, 0.29, 0.5],
    cursor_color: [0.99, 0.99, 0.98, 0.9],
    normal_colors: [
        (45, 42, 46),
        (255, 97, 136),
        (169, 220, 118),
        (255, 216, 102),
        (120, 220, 232),
        (171, 157, 242),
        (120, 220, 232),
        (252, 252, 250),
    ],
    bright_colors: [
        (114, 109, 118),
        (255, 97, 136),
        (169, 220, 118),
        (255, 216, 102),
        (120, 220, 232),
        (171, 157, 242),
        (120, 220, 232),
        (252, 252, 250),
    ],
    accent: [1.0, 0.85, 0.40, 1.0],
    muted: [0.60, 0.58, 0.62, 1.0],
    border_radius: 6.0,
    padding: 12.0,
};

// ADI theme - violet/blue space nebula palette matching intro animation
const ADI: ThemeColors = ThemeColors {
    name: "ADI",
    background: [0.04, 0.03, 0.08, 1.0], // Deep violet-black
    foreground: (220, 215, 245),         // Soft lavender white
    selection: [0.25, 0.15, 0.45, 0.5],  // Violet selection
    cursor_color: [0.7, 0.5, 0.95, 0.9], // Bright violet cursor
    normal_colors: [
        (25, 20, 45),    // black - deep violet
        (230, 100, 140), // red - soft pink-red
        (130, 200, 180), // green - teal-mint
        (220, 180, 130), // yellow - soft gold
        (100, 140, 220), // blue - electric blue
        (180, 120, 220), // magenta - bright violet
        (100, 180, 220), // cyan - bright cyan
        (200, 195, 230), // white - lavender
    ],
    bright_colors: [
        (80, 70, 120),   // bright black - muted violet
        (255, 130, 170), // bright red - hot pink
        (160, 230, 210), // bright green - bright teal
        (250, 210, 160), // bright yellow - bright gold
        (130, 170, 250), // bright blue - bright electric
        (210, 150, 250), // bright magenta - bright violet
        (130, 210, 250), // bright cyan - bright cyan
        (240, 235, 255), // bright white - pure lavender white
    ],
    accent: [0.55, 0.35, 0.85, 1.0], // Violet accent
    muted: [0.45, 0.40, 0.60, 1.0],  // Muted violet
    border_radius: 8.0,
    padding: 12.0,
};

impl Theme {
    pub fn tokyo_night() -> Self {
        TOKYO_NIGHT.build()
    }

    pub fn dracula() -> Self {
        DRACULA.build()
    }

    pub fn one_dark() -> Self {
        ONE_DARK.build()
    }

    pub fn catppuccin_mocha() -> Self {
        CATPPUCCIN_MOCHA.build()
    }

    pub fn gruvbox_dark() -> Self {
        GRUVBOX_DARK.build()
    }

    pub fn nord() -> Self {
        NORD.build()
    }

    pub fn solarized_dark() -> Self {
        SOLARIZED_DARK.build()
    }

    pub fn monokai_pro() -> Self {
        MONOKAI_PRO.build()
    }

    pub fn adi() -> Self {
        ADI.build()
    }

    /// Get all available theme names
    pub fn available_themes() -> Vec<&'static str> {
        vec![
            "ADI",
            "Tokyo Night",
            "Dracula",
            "One Dark",
            "Catppuccin Mocha",
            "Gruvbox Dark",
            "Nord",
            "Solarized Dark",
            "Monokai Pro",
        ]
    }

    /// Get theme by name
    pub fn by_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "adi" => Some(Self::adi()),
            "tokyo night" | "tokyonight" => Some(Self::tokyo_night()),
            "dracula" => Some(Self::dracula()),
            "one dark" | "onedark" => Some(Self::one_dark()),
            "catppuccin mocha" | "catppuccin" => Some(Self::catppuccin_mocha()),
            "gruvbox dark" | "gruvbox" => Some(Self::gruvbox_dark()),
            "nord" => Some(Self::nord()),
            "solarized dark" | "solarized" => Some(Self::solarized_dark()),
            "monokai pro" | "monokai" => Some(Self::monokai_pro()),
            _ => None,
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::adi()
    }
}
