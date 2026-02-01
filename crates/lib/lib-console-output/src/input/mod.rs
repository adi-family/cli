// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Interactive input components for console.
//!
//! Supports three modes:
//! - Interactive terminal: Full keyboard navigation
//! - Non-interactive: Fallback to simple prompts or defaults
//! - JSON stream: Request/response protocol for remote input

mod confirm;
mod text;
mod multiselect;
mod password;
mod select;
mod types;

pub use confirm::{confirm, Confirm};
pub use text::{text_input, Input};
pub use multiselect::{multi_select, MultiSelect};
pub use password::{password, Password};
pub use select::{select, Select};
pub use types::{InputEvent, InputRequest, SelectOption};
