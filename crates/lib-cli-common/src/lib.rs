// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Common CLI utilities for ADI tools.
//!
//! This library provides shared functionality for ADI CLI applications:
//! - Output formatting with consistent styling
//! - Project path handling
//! - Logging setup
//! - CLI runner with error handling

mod logging;
pub mod output;
mod project;
mod runner;

pub use logging::{setup_logging, setup_logging_quiet};
pub use output::{
    error_icon, print_empty, print_error, print_info, print_json, print_success, print_warning,
    success_icon, warning_icon, OutputFormat,
};
pub use project::ProjectPath;
pub use runner::run_cli;
