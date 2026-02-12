// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Console output abstraction with support for text and JSON stream modes.
//!
//! This library provides a unified interface for console output that can switch
//! between human-readable text mode and JSON stream mode based on environment
//! variables. The JSON stream mode is designed for WebRTC/cloud consumption.
//!
//! # Modes
//!
//! - **Text mode** (default): Human-readable output with colors and icons
//! - **JSON stream mode** (silk mode): Single-line JSON objects for machine parsing
//!
//! # Environment Variables
//!
//! - `SILK_MODE=true|1` - Enable JSON stream mode
//! - `NO_COLOR` - Disable colors in text mode
//! - `VERBOSE=true|1` - Show trace-level output
//! - `QUIET=true|1` - Show only errors
//!
//! # Usage
//!
//! ## Simple output
//!
//! ```rust,no_run
//! use lib_console_output::{info, success, error, warn};
//!
//! info("Starting operation...");
//! success("Operation completed");
//! warn("Resource usage high");
//! error("Connection failed");
//! ```
//!
//! ## With format strings (macros)
//!
//! ```rust,no_run
//! use lib_console_output::{out_info, out_error, Level};
//!
//! let count = 5;
//! out_info!("Processed {} files", count);
//! out_error!("Failed after {} retries", 3);
//! ```
//!
//! ## Structured messages
//!
//! ```rust,no_run
//! use lib_console_output::{MessageBuilder, Level, message};
//!
//! let msg = MessageBuilder::new(Level::Info, "User action")
//!     .field("user_id", "123")
//!     .field("action", "login")
//!     .build();
//! message(&msg);
//! ```
//!
//! ## Custom console instance
//!
//! ```rust,no_run
//! use lib_console_output::{Console, ConsoleConfig, OutputMode, Level};
//!
//! let config = ConsoleConfig::json_stream()
//!     .with_min_level(Level::Debug);
//! let console = Console::new(config);
//! console.info("This will be JSON");
//! ```
//!
//! # JSON Stream Output
//!
//! In JSON stream mode (`SILK_MODE=true`), each message is output as a single
//! JSON line that can be easily parsed by WebRTC receivers:
//!
//! ```json
//! {"timestamp":"2025-01-31T12:00:00Z","level":"info","message":"Starting..."}
//! {"timestamp":"2025-01-31T12:00:01Z","level":"success","message":"Done","fields":{"count":5}}
//! ```
//!
//! # Progress Indicators
//!
//! The library provides progress indicators that adapt to the output mode:
//!
//! ## Spinner (indeterminate progress)
//!
//! ```rust,no_run
//! use lib_console_output::progress::spinner;
//!
//! let mut sp = spinner("Loading...");
//! // ... do work, optionally call sp.tick() in a loop ...
//! sp.success(Some("Loaded successfully"));
//! ```
//!
//! ## Progress Bar (determinate progress)
//!
//! ```rust,no_run
//! use lib_console_output::progress::progress_bar;
//!
//! let mut pb = progress_bar(100, "Downloading");
//! for i in 0..100 {
//!     pb.inc();
//!     // ... do work ...
//! }
//! pb.success(None);
//! ```
//!
//! ## Step Progress (multi-step operations)
//!
//! ```rust,no_run
//! use lib_console_output::progress::steps;
//!
//! let mut sp = steps(3, "Initializing");
//! sp.next_step("Connecting to server");
//! sp.next_step("Authenticating");
//! sp.next_step("Loading data");
//! sp.success(Some("Ready"));
//! ```
//!
//! In JSON stream mode, progress events are emitted as structured JSON:
//!
//! ```json
//! {"type":"start","id":"abc123","message":"Loading...","total":null,"timestamp":"..."}
//! {"type":"update","id":"abc123","current":50,"total":100,"percent":50.0,"timestamp":"..."}
//! {"type":"complete","id":"abc123","message":"Done","timestamp":"..."}
//! ```
//!
//! # Interactive Input
//!
//! The library provides input components that adapt to the output mode:
//!
//! ## Select (single choice)
//!
//! ```rust,no_run
//! use lib_console_output::input::{Select, SelectOption};
//!
//! let choice = Select::new("Choose a color")
//!     .option(SelectOption::new("Red", "red"))
//!     .option(SelectOption::new("Green", "green"))
//!     .option(SelectOption::new("Blue", "blue"))
//!     .run();
//! ```
//!
//! ## MultiSelect (multiple choices)
//!
//! ```rust,no_run
//! use lib_console_output::input::{MultiSelect, SelectOption};
//!
//! let choices = MultiSelect::new("Choose toppings")
//!     .option(SelectOption::new("Cheese", "cheese"))
//!     .option(SelectOption::new("Pepperoni", "pepperoni"))
//!     .option(SelectOption::new("Mushrooms", "mushrooms"))
//!     .run();
//! ```
//!
//! ## Confirm (yes/no)
//!
//! ```rust,no_run
//! use lib_console_output::input::Confirm;
//!
//! if Confirm::new("Continue?").default(true).run() == Some(true) {
//!     // proceed
//! }
//! ```
//!
//! ## Input (text)
//!
//! ```rust,no_run
//! use lib_console_output::input::Input;
//!
//! let name = Input::new("Enter your name")
//!     .default("Anonymous")
//!     .run();
//! ```
//!
//! ## Password (hidden)
//!
//! ```rust,no_run
//! use lib_console_output::input::Password;
//!
//! let secret = Password::new("Enter password")
//!     .confirm("Confirm password")
//!     .run();
//! ```
//!
//! In JSON stream mode, input requests are emitted and responses are read:
//!
//! ```json
//! {"type":"select","id":"input_abc","prompt":"Choose","options":[...],"timestamp":"..."}
//! // Response: {"type":"select_response","id":"input_abc","index":0,"timestamp":"..."}
//! ```

pub mod blocks;
mod config;
mod console;
pub mod input;
mod level;
#[macro_use]
mod macros;
mod message;
mod mode;
pub mod progress;
mod style;
pub mod theme;

pub use config::{ConsoleConfig, NO_COLOR_ENV, QUIET_ENV, SILK_MODE_ENV, VERBOSE_ENV};
pub use theme::ADI_THEME_ENV;
pub use console::{
    console, data, debug, error, info, init, is_initialized, message, success, trace, warn, Console,
};
pub use input::{
    confirm, multi_select, password, select, text_input, Confirm, Input, InputEvent, InputRequest,
    MultiSelect, Password, Select, SelectOption,
};
pub use level::Level;
pub use message::{MessageBuilder, OutputMessage};
pub use mode::OutputMode;
pub use progress::{
    is_interactive, progress_bar, spinner, steps, MultiProgress, ProgressBar, ProgressEvent,
    Spinner, StepProgress,
};
pub use style::{format_text_line, icons, level_icon, level_prefix, styled_message};
pub use blocks::{
    Card, Columns, KeyValue, List, LiveKeyValue, LiveTable, Renderable, Section, Table, LiveHandle,
};
