//! Input field components
//!
//! Styled text input fields for commands and search.
//! Uses dual font system: Inter (sans-serif) for UI, JetBrains Mono for code.

use crate::fonts::CODE_FONT;
use iced::widget::{text_input, TextInput};
use iced::Length;

/// Create a command input field (terminal-style, uses monospace font)
pub fn command_input<'a, M: Clone + 'a>(
    placeholder: &str,
    value: &str,
    id: text_input::Id,
    on_input: impl Fn(String) -> M + 'a,
    on_submit: M,
) -> TextInput<'a, M> {
    text_input(placeholder, value)
        .id(id)
        .on_input(on_input)
        .on_submit(on_submit)
        .padding(10)
        .size(14)
        .font(CODE_FONT)
        .width(Length::Fill)
}

/// Create a search input field (for command palette, etc.)
pub fn search_input<'a, M: Clone + 'a>(
    placeholder: &str,
    value: &str,
    id: text_input::Id,
    on_input: impl Fn(String) -> M + 'a,
    on_submit: M,
) -> TextInput<'a, M> {
    text_input(placeholder, value)
        .id(id)
        .on_input(on_input)
        .on_submit(on_submit)
        .padding(12)
        .size(16)
        .width(Length::Fill)
}

/// Configuration for input styling
pub struct InputConfig {
    pub padding: u16,
    pub size: f32,
    pub monospace: bool,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            padding: 10,
            size: 14.0,
            monospace: false,
        }
    }
}

impl InputConfig {
    pub fn command() -> Self {
        Self {
            padding: 10,
            size: 14.0,
            monospace: true,
        }
    }

    pub fn search() -> Self {
        Self {
            padding: 12,
            size: 16.0,
            monospace: false,
        }
    }
}

/// Create a styled input with configuration
pub fn styled_input<'a, M: Clone + 'a>(
    placeholder: &str,
    value: &str,
    config: InputConfig,
    on_input: impl Fn(String) -> M + 'a,
) -> TextInput<'a, M> {
    let input = text_input(placeholder, value)
        .on_input(on_input)
        .padding(config.padding)
        .size(config.size)
        .width(Length::Fill);

    if config.monospace {
        input.font(CODE_FONT)
    } else {
        input
    }
}
