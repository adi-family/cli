// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Common types for input components.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An option for select/multiselect components.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption<T: Clone> {
    /// Display label.
    pub label: String,
    /// The value returned when selected.
    pub value: T,
    /// Optional description shown below the label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this option is disabled.
    #[serde(default)]
    pub disabled: bool,
}

impl<T: Clone> SelectOption<T> {
    /// Create a new option with just a label and value.
    pub fn new(label: impl Into<String>, value: T) -> Self {
        Self {
            label: label.into(),
            value,
            description: None,
            disabled: false,
        }
    }

    /// Add a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Mark as disabled.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

/// Create an option from a simple string (label = value).
impl From<String> for SelectOption<String> {
    fn from(s: String) -> Self {
        Self::new(s.clone(), s)
    }
}

impl From<&str> for SelectOption<String> {
    fn from(s: &str) -> Self {
        Self::new(s, s.to_string())
    }
}

/// Input request events for JSON stream mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputRequest {
    /// Request a single selection.
    Select {
        id: String,
        prompt: String,
        options: Vec<SelectOptionJson>,
        default: Option<usize>,
        timestamp: DateTime<Utc>,
    },
    /// Request multiple selections.
    MultiSelect {
        id: String,
        prompt: String,
        options: Vec<SelectOptionJson>,
        defaults: Vec<usize>,
        min: Option<usize>,
        max: Option<usize>,
        timestamp: DateTime<Utc>,
    },
    /// Request yes/no confirmation.
    Confirm {
        id: String,
        prompt: String,
        default: Option<bool>,
        timestamp: DateTime<Utc>,
    },
    /// Request text input.
    Input {
        id: String,
        prompt: String,
        default: Option<String>,
        placeholder: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// Request password input.
    Password {
        id: String,
        prompt: String,
        timestamp: DateTime<Utc>,
    },
}

/// Simplified option for JSON serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOptionJson {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub disabled: bool,
}

impl<T: Clone> From<&SelectOption<T>> for SelectOptionJson {
    fn from(opt: &SelectOption<T>) -> Self {
        Self {
            label: opt.label.clone(),
            description: opt.description.clone(),
            disabled: opt.disabled,
        }
    }
}

impl InputRequest {
    /// Serialize to JSON line.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"type":"error"}"#.to_string())
    }
}

/// Input response events for JSON stream mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputEvent {
    /// Single selection response.
    SelectResponse {
        id: String,
        index: usize,
        timestamp: DateTime<Utc>,
    },
    /// Multiple selection response.
    MultiSelectResponse {
        id: String,
        indices: Vec<usize>,
        timestamp: DateTime<Utc>,
    },
    /// Confirmation response.
    ConfirmResponse {
        id: String,
        value: bool,
        timestamp: DateTime<Utc>,
    },
    /// Text input response.
    InputResponse {
        id: String,
        value: String,
        timestamp: DateTime<Utc>,
    },
    /// Input cancelled.
    Cancelled {
        id: String,
        timestamp: DateTime<Utc>,
    },
}

impl InputEvent {
    /// Serialize to JSON line.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"type":"error"}"#.to_string())
    }

    /// Try to parse from JSON.
    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

/// Generate a simple unique ID for input requests.
pub(crate) fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    format!("input_{:x}{:x}", now.as_nanos(), std::process::id())
}
