//! Card container components
//!
//! Styled containers for blocks, modals, and panels.

use crate::colors::UiColors;
use iced::widget::{container, Container};
use iced::{Color, Element, Length};

/// Card style variant
#[derive(Debug, Clone, Copy, Default)]
pub enum CardStyle {
    #[default]
    Default,
    Running,
    Success,
    Error,
    Interactive,
    System,
}

/// Card component configuration
pub struct Card {
    pub style: CardStyle,
    pub padding: u16,
    pub border_radius: f32,
}

impl Card {
    pub fn new() -> Self {
        Self {
            style: CardStyle::Default,
            padding: 16,
            border_radius: 8.0,
        }
    }

    pub fn style(mut self, style: CardStyle) -> Self {
        self.style = style;
        self
    }

    pub fn padding(mut self, padding: u16) -> Self {
        self.padding = padding;
        self
    }

    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a styled card container
pub fn card<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    config: Card,
    colors: UiColors,
) -> Container<'a, M> {
    let border_color = match config.style {
        CardStyle::Default => colors.border,
        CardStyle::Running => colors.border_running,
        CardStyle::Success => colors.border_success,
        CardStyle::Error => colors.border_error,
        CardStyle::Interactive => colors.border_interactive,
        CardStyle::System => colors.border,
    };

    let bg_color = match config.style {
        CardStyle::System => colors.system_bg,
        _ => colors.block_bg,
    };

    let radius = config.border_radius;
    let padding = config.padding;

    container(content)
        .padding(padding)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(bg_color)),
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: radius.into(),
            },
            ..Default::default()
        })
}

/// Create a modal card (floating panel with shadow)
pub fn modal_card<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    width: f32,
    colors: UiColors,
) -> Container<'a, M> {
    let bg = colors.block_bg;
    let border = colors.border_running;

    container(content)
        .width(Length::Fixed(width))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border {
                color: border,
                width: 2.0,
                radius: 12.0.into(),
            },
            shadow: iced::Shadow {
                color: Color::BLACK,
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 20.0,
            },
            ..Default::default()
        })
}

/// Create a backdrop overlay (semi-transparent background)
pub fn backdrop<'a, M: 'a>(content: impl Into<Element<'a, M>>, opacity: f32) -> Container<'a, M> {
    let backdrop_color = Color {
        a: opacity,
        ..Color::BLACK
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(backdrop_color)),
            ..Default::default()
        })
}

/// Create a section container (for settings sections, etc.)
pub fn section<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    colors: UiColors,
) -> Container<'a, M> {
    let bg = colors.system_bg;
    let border = colors.border;

    container(content)
        .padding(16)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border {
                color: border,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
}

/// Create an inline code/command display
pub fn code_block<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    colors: UiColors,
) -> Container<'a, M> {
    let bg = colors.system_bg;

    container(content)
        .padding([4, 8])
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
}
