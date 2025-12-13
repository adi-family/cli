// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

use console::style;
use serde::Serialize;

use super::icons::{info_icon, success_icon, warning_icon};

/// Print a success message with green checkmark.
pub fn print_success(message: &str) {
    println!("{} {}", success_icon(), message);
}

/// Print an error message with red styling.
pub fn print_error(message: &str) {
    eprintln!("{} {}", style("Error:").red().bold(), message);
}

/// Print a warning message with yellow styling.
pub fn print_warning(message: &str) {
    println!("{} {}", warning_icon(), message);
}

/// Print an info message with blue styling.
pub fn print_info(message: &str) {
    println!("{} {}", info_icon(), message);
}

/// Print a dimmed "no results" message.
pub fn print_empty(message: &str) {
    println!("{}", style(message).dim());
}

/// Print a header/title in bold.
pub fn print_header(title: &str) {
    println!("{}", style(title).bold());
}

/// Print a count message like "Found 5 results:".
pub fn print_count(prefix: &str, count: usize, suffix: &str) {
    println!("{} {} {}:", style(prefix).dim(), count, suffix);
}

/// Print data as pretty JSON.
pub fn print_json<T: Serialize>(data: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(data)?);
    Ok(())
}

/// Print a location reference (file:line format).
#[allow(dead_code)]
pub fn print_location(file: &str, line: usize) {
    println!("  {}:{}", file, line);
}

/// Helper for printing indented content.
#[allow(dead_code)]
pub fn print_indented(indent: usize, content: &str) {
    let prefix = "  ".repeat(indent);
    println!("{}{}", prefix, content);
}
