// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Logging setup utilities.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize logging with the given verbosity level.
///
/// - `verbose = true`: Sets log level to debug
/// - `verbose = false`: Sets log level to info
///
/// Respects `RUST_LOG` environment variable if set.
pub fn setup_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}

/// Initialize logging with a default warning level (minimal output).
pub fn setup_logging_quiet() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}
