//! Font system for iced UI
//!
//! Triple font system: sans-serif for UI, monospace for code, icons for UI elements.

use iced::Font;

/// Sans-serif font for UI elements (Inter)
pub const UI_FONT: Font = Font::with_name("Inter");

/// Sans-serif medium weight for emphasis
pub const UI_FONT_MEDIUM: Font = Font {
    family: iced::font::Family::Name("Inter"),
    weight: iced::font::Weight::Medium,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

/// Sans-serif semi-bold for headers
pub const UI_FONT_SEMIBOLD: Font = Font {
    family: iced::font::Family::Name("Inter"),
    weight: iced::font::Weight::Semibold,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

/// Sans-serif bold for logos and emphasis
pub const UI_FONT_BOLD: Font = Font {
    family: iced::font::Family::Name("Inter"),
    weight: iced::font::Weight::Bold,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

/// Monospace font for code/terminal (JetBrains Mono)
pub const CODE_FONT: Font = Font::with_name("JetBrains Mono");

/// Monospace bold for emphasis in code
pub const CODE_FONT_BOLD: Font = Font {
    family: iced::font::Family::Name("JetBrains Mono"),
    weight: iced::font::Weight::Bold,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

/// Icon font for UI elements (Phosphor Icons)
pub const ICON_FONT: Font = Font::with_name("Phosphor");

/// Icon font bold weight
pub const ICON_FONT_BOLD: Font = Font {
    family: iced::font::Family::Name("Phosphor-Bold"),
    weight: iced::font::Weight::Bold,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

/// Create a custom font with the given name
pub const fn custom_font(name: &'static str) -> Font {
    Font::with_name(name)
}

/// Create a bold variant of a named font
pub const fn bold_font(name: &'static str) -> Font {
    Font {
        family: iced::font::Family::Name(name),
        weight: iced::font::Weight::Bold,
        stretch: iced::font::Stretch::Normal,
        style: iced::font::Style::Normal,
    }
}
