//! Status pill components
//!
//! Pill-shaped indicators for git status, environment, running processes, etc.

use crate::colors::UiColors;
use iced::widget::{button, container, horizontal_space, row, text, tooltip};
use iced::{Color, Element};

/// Create a generic status pill (non-interactive)
pub fn status_pill<'a, M: 'a>(label: impl Into<String>, color: Color) -> Element<'a, M> {
    let label_text = label.into();
    let pill_color = color;

    container(text(format!(" {}", label_text)).size(11).color(pill_color))
        .padding([3, 8])
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(Color {
                a: 0.2,
                ..pill_color
            })),
            border: iced::Border {
                color: pill_color,
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Create a git status pill
pub fn git_pill<'a, M: 'a>(
    branch: &str,
    uncommitted: usize,
    ahead: usize,
    behind: usize,
    colors: UiColors,
) -> Element<'a, M> {
    let color = if uncommitted > 0 {
        colors.status_warning
    } else if ahead > 0 || behind > 0 {
        colors.status_info
    } else {
        colors.status_success
    };

    // Format: "branch +ahead -behind *uncommitted"
    let mut label = format!(" {}", branch);
    if ahead > 0 {
        label.push_str(&format!(" +{}", ahead));
    }
    if behind > 0 {
        label.push_str(&format!(" -{}", behind));
    }
    if uncommitted > 0 {
        label.push_str(&format!(" *{}", uncommitted));
    }

    status_pill(label, color)
}

/// Create a running processes pill (clickable to cancel)
pub fn running_pill<'a, M: Clone + 'a>(
    count: usize,
    colors: UiColors,
    on_cancel: M,
) -> Element<'a, M> {
    if count == 0 {
        return horizontal_space().width(0).into();
    }

    let pill_color = colors.status_running;
    let running_label = text(format!(" {} running", count))
        .size(11)
        .color(pill_color);

    tooltip(
        button(running_label)
            .on_press(on_cancel)
            .padding([3, 8])
            .style(move |_, _| button::Style {
                background: Some(iced::Background::Color(Color {
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
            }),
        "Click to cancel all (Ctrl+C)",
        tooltip::Position::Bottom,
    )
    .into()
}

/// Create an environment indicator pill (prod/staging/dev)
pub fn env_pill<'a, M: 'a>(
    name: &str,
    icon: &str,
    text_color: [f32; 4],
    bg_color: [f32; 4],
) -> Element<'a, M> {
    let env_color = Color::from_rgba(text_color[0], text_color[1], text_color[2], text_color[3]);
    let env_bg = Color::from_rgba(bg_color[0], bg_color[1], bg_color[2], bg_color[3]);

    let env_label = text(format!("{} {}", icon, name)).size(11).color(env_color);

    container(env_label)
        .padding([3, 8])
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(env_bg)),
            border: iced::Border {
                color: env_color,
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Create a system stats pill (CPU/memory sparklines)
pub fn stats_pill<'a, M: 'a>(
    cpu_spark: &str,
    cpu_pct: u32,
    mem_spark: &str,
    mem_pct: u32,
    colors: UiColors,
) -> Element<'a, M> {
    let stats_content = row![
        text(format!("CPU {} {}%", cpu_spark, cpu_pct))
            .size(10)
            .color(colors.muted_text),
        text(" | ").size(10).color(colors.muted_text),
        text(format!("MEM {} {}%", mem_spark, mem_pct))
            .size(10)
            .color(colors.muted_text),
    ]
    .spacing(0);

    let border_color = colors.border;
    let muted = colors.muted_text;

    container(stats_content)
        .padding([3, 8])
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(Color { a: 0.3, ..muted })),
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Create an empty placeholder (for conditional rendering)
pub fn empty_pill<'a, M: 'a>() -> Element<'a, M> {
    horizontal_space().width(0).into()
}
