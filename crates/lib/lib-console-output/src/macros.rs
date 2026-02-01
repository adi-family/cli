// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Convenience macros for console output.

/// Output a trace message with format support.
///
/// # Example
/// ```ignore
/// out_trace!("Processing file: {}", filename);
/// ```
#[macro_export]
macro_rules! out_trace {
    ($($arg:tt)*) => {
        $crate::trace(&format!($($arg)*))
    };
}

/// Output a debug message with format support.
///
/// # Example
/// ```ignore
/// out_debug!("Cache hit for key: {}", key);
/// ```
#[macro_export]
macro_rules! out_debug {
    ($($arg:tt)*) => {
        $crate::debug(&format!($($arg)*))
    };
}

/// Output an info message with format support.
///
/// # Example
/// ```ignore
/// out_info!("Server started on port {}", port);
/// ```
#[macro_export]
macro_rules! out_info {
    ($($arg:tt)*) => {
        $crate::info(&format!($($arg)*))
    };
}

/// Output a success message with format support.
///
/// # Example
/// ```ignore
/// out_success!("Created {} files", count);
/// ```
#[macro_export]
macro_rules! out_success {
    ($($arg:tt)*) => {
        $crate::success(&format!($($arg)*))
    };
}

/// Output a warning message with format support.
///
/// # Example
/// ```ignore
/// out_warn!("Deprecated API used: {}", api_name);
/// ```
#[macro_export]
macro_rules! out_warn {
    ($($arg:tt)*) => {
        $crate::warn(&format!($($arg)*))
    };
}

/// Output an error message with format support.
///
/// # Example
/// ```ignore
/// out_error!("Failed to connect: {}", err);
/// ```
#[macro_export]
macro_rules! out_error {
    ($($arg:tt)*) => {
        $crate::error(&format!($($arg)*))
    };
}

/// Output a message at a specific level with format support.
///
/// # Example
/// ```ignore
/// out!(Level::Info, "Processing {} items", count);
/// ```
#[macro_export]
macro_rules! out {
    ($level:expr, $($arg:tt)*) => {
        $crate::console().output($level, &format!($($arg)*))
    };
}

/// Create a structured message with fields.
///
/// # Example
/// ```ignore
/// let msg = out_msg!(Level::Info, "User action",
///     "user_id" => user.id,
///     "action" => "login"
/// );
/// message(&msg);
/// ```
#[macro_export]
macro_rules! out_msg {
    ($level:expr, $message:expr $(, $key:expr => $value:expr)*) => {{
        let mut builder = $crate::MessageBuilder::new($level, $message);
        $(
            builder = builder.field($key, $value);
        )*
        builder.build()
    }};
}
