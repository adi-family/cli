// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! CLI runner with error handling.

use crate::print_error;

/// Run a CLI function with consistent error handling.
///
/// On error, prints the error message in red and exits with code 1.
pub fn run_cli<F>(f: F)
where
    F: FnOnce() -> anyhow::Result<()>,
{
    if let Err(e) = f() {
        print_error(&format!("{}", e));
        std::process::exit(1);
    }
}
