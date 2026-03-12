//! Button components with consistent styling
//!
//! Provides primary, secondary, text, and icon button variants.

use crate::colors::UiColors;
use iced::widget::{button, text, tooltip, Button};
use iced::Element;

/// Button variant for styling
#[derive(Debug, Clone, Copy, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Text,
    Danger,
}

/// Icon button configuration
pub struct IconButton {
    pub icon: &'static str,
    pub tooltip_text: Option<&'static str>,
    pub size: f32,
}

impl IconButton {
    pub fn new(icon: &'static str) -> Self {
        Self {
            icon,
            tooltip_text: None,
            size: 14.0,
        }
    }

    pub fn tooltip(mut self, text: &'static str) -> Self {
        self.tooltip_text = Some(text);
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

/// Create a primary action button
pub fn primary_button<'a, M: Clone + 'a>(
    label: impl Into<String>,
    colors: UiColors,
    on_press: Option<M>,
) -> Element<'a, M> {
    let label_text = label.into();
    let btn = button(text(label_text).size(colors.label_size))
        .padding([6, 12])
        .style(button::primary);

    match on_press {
        Some(msg) => btn.on_press(msg).into(),
        None => btn.into(),
    }
}

/// Create a secondary action button
pub fn secondary_button<'a, M: Clone + 'a>(
    label: impl Into<String>,
    colors: UiColors,
    on_press: Option<M>,
) -> Element<'a, M> {
    let label_text = label.into();
    let btn = button(text(label_text).size(colors.label_size))
        .padding([6, 12])
        .style(button::secondary);

    match on_press {
        Some(msg) => btn.on_press(msg).into(),
        None => btn.into(),
    }
}

/// Create a text-style button (minimal styling)
pub fn text_button<'a, M: Clone + 'a>(
    label: impl Into<String>,
    colors: UiColors,
    on_press: Option<M>,
) -> Element<'a, M> {
    let label_text = label.into();
    let btn = button(
        text(label_text)
            .size(colors.label_size)
            .color(colors.muted_text),
    )
    .padding([4, 8])
    .style(button::text);

    match on_press {
        Some(msg) => btn.on_press(msg).into(),
        None => btn.into(),
    }
}

/// Create an icon button with optional tooltip
pub fn icon_button<'a, M: Clone + 'a>(
    config: IconButton,
    colors: UiColors,
    on_press: Option<M>,
) -> Element<'a, M> {
    let btn_content = text(config.icon).size(config.size).color(colors.muted_text);

    let btn = button(btn_content).padding([4, 8]).style(button::text);

    let btn: Button<'a, M> = match on_press {
        Some(msg) => btn.on_press(msg),
        None => btn,
    };

    match config.tooltip_text {
        Some(tip) => tooltip(btn, tip, tooltip::Position::Bottom).into(),
        None => btn.into(),
    }
}

/// Create a tab-style button (for session tabs)
pub fn tab_button<'a, M: Clone + 'a>(
    label: impl Into<String>,
    is_active: bool,
    on_press: M,
) -> Element<'a, M> {
    let label_text = label.into();
    button(text(label_text).size(12))
        .on_press(on_press)
        .padding([4, 8])
        .style(if is_active {
            button::primary
        } else {
            button::secondary
        })
        .into()
}

/// Create a pill-style button (clickable status indicator)
pub fn pill_button<'a, M: Clone + 'a>(
    label: impl Into<String>,
    color: iced::Color,
    on_press: Option<M>,
) -> Element<'a, M> {
    let label_text = label.into();
    let pill_color = color;

    let btn = button(text(label_text).size(11).color(pill_color))
        .padding([3, 8])
        .style(move |_, _| button::Style {
            background: Some(iced::Background::Color(iced::Color {
                a: 0.2,
                ..pill_color
            })),
            text_color: pill_color,
            border: iced::Border {
                color: pill_color,
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        });

    match on_press {
        Some(msg) => btn.on_press(msg).into(),
        None => btn.into(),
    }
}

/// Create a close button (X icon)
pub fn close_button<'a, M: Clone + 'a>(colors: UiColors, on_press: M) -> Element<'a, M> {
    button(text("x").size(14).color(colors.muted_text))
        .on_press(on_press)
        .padding([2, 6])
        .style(button::text)
        .into()
}

/// Create a small action button for block action bars
pub fn action_button<'a, M: Clone + 'a>(
    icon: &'static str,
    tooltip_text: &'static str,
    colors: UiColors,
    on_press: M,
) -> Element<'a, M> {
    tooltip(
        button(text(icon).size(12).color(colors.muted_text))
            .on_press(on_press)
            .padding([4, 8])
            .style(button::text),
        tooltip_text,
        tooltip::Position::Bottom,
    )
    .into()
}

/// Create a header button that spans full height with hover animation
pub fn header_button<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    colors: UiColors,
    on_press: M,
) -> Element<'a, M> {
    let hover_bg = colors.system_bg;

    button(
        iced::widget::container(content)
            .align_y(iced::Alignment::Center)
            .height(iced::Length::Fill),
    )
    .on_press(on_press)
    .padding([0, 12])
    .height(iced::Length::Fill)
    .style(move |_, status| {
        let bg = match status {
            button::Status::Hovered => hover_bg,
            _ => iced::Color::TRANSPARENT,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            border: iced::Border::default(),
            ..Default::default()
        }
    })
    .into()
}
