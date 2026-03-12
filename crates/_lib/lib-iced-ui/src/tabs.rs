//! Tab bar components
//!
//! Session tabs with activity indicators and add button.

use iced::widget::{button, row, text, Row};
use iced::Element;

/// Tab item data
pub struct TabItem {
    pub index: usize,
    pub is_active: bool,
    pub has_activity: bool,
}

impl TabItem {
    pub fn new(index: usize, is_active: bool, has_activity: bool) -> Self {
        Self {
            index,
            is_active,
            has_activity,
        }
    }
}

/// Create a session tab bar
pub fn session_tabs<'a, M: Clone + 'a>(
    tabs: &[TabItem],
    on_select: impl Fn(usize) -> M + 'a,
    on_new: M,
) -> Element<'a, M> {
    let mut tabs_row: Row<'a, M> = row![].spacing(2);

    for tab in tabs {
        let tab_label = if tab.has_activity {
            format!(" {} * ", tab.index + 1)
        } else {
            format!(" {} ", tab.index + 1)
        };

        let tab_button = button(text(tab_label).size(12))
            .on_press(on_select(tab.index))
            .padding([4, 8])
            .style(if tab.is_active {
                iced::widget::button::primary
            } else {
                iced::widget::button::secondary
            });

        tabs_row = tabs_row.push(tab_button);
    }

    // Add new tab button
    tabs_row = tabs_row.push(
        button(text("+").size(12))
            .on_press(on_new)
            .padding([4, 8])
            .style(iced::widget::button::text),
    );

    tabs_row.into()
}

/// Create a simple tab bar (without activity indicators)
pub fn simple_tabs<'a, M: Clone + 'a>(
    labels: &[&'a str],
    active_index: usize,
    on_select: impl Fn(usize) -> M + 'a,
) -> Element<'a, M> {
    let mut tabs_row: Row<'a, M> = row![].spacing(2);

    for (idx, label) in labels.iter().enumerate() {
        let is_active = idx == active_index;
        let tab_button = button(text(*label).size(12))
            .on_press(on_select(idx))
            .padding([6, 12])
            .style(if is_active {
                iced::widget::button::primary
            } else {
                iced::widget::button::secondary
            });

        tabs_row = tabs_row.push(tab_button);
    }

    tabs_row.into()
}

/// Create sidebar navigation tabs
pub fn nav_tabs<'a, M: Clone + 'a, T: PartialEq + Copy>(
    items: &[(T, &'a str)],
    active: T,
    on_select: impl Fn(T) -> M + 'a,
) -> Element<'a, M> {
    let mut tabs_row: Row<'a, M> = row![].spacing(4);

    for (value, label) in items {
        let is_active = *value == active;
        let tab_button = button(text(*label).size(11))
            .on_press(on_select(*value))
            .padding([6, 12])
            .style(if is_active {
                iced::widget::button::primary
            } else {
                iced::widget::button::text
            });

        tabs_row = tabs_row.push(tab_button);
    }

    tabs_row.into()
}
