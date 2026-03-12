// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Output formatting utilities for consistent CLI styling.

mod format;
mod icons;
mod print;

pub use format::OutputFormat;
pub use icons::{error_icon, info_icon, success_icon, warning_icon};
pub use print::{
    print_count, print_empty, print_error, print_header, print_info, print_json, print_success,
    print_warning,
};
