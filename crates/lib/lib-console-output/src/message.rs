// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Message types for structured output.

use crate::Level;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A structured output message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMessage {
    /// ISO 8601 timestamp.
    pub timestamp: DateTime<Utc>,
    /// Message level.
    pub level: Level,
    /// Message content.
    pub message: String,
    /// Optional structured fields.
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub fields: HashMap<String, serde_json::Value>,
    /// Optional source context (file, function, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl OutputMessage {
    /// Create a new output message.
    pub fn new(level: Level, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            message: message.into(),
            fields: HashMap::new(),
            source: None,
        }
    }

    /// Add a field to the message.
    pub fn with_field(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.fields.insert(key.into(), v);
        }
        self
    }

    /// Add multiple fields to the message.
    pub fn with_fields(mut self, fields: HashMap<String, serde_json::Value>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// Set the source context.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Serialize to JSON (single line, no trailing newline).
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            format!(
                r#"{{"timestamp":"{}","level":"{}","message":"{}"}}"#,
                self.timestamp.to_rfc3339(),
                self.level,
                self.message.replace('"', "\\\"")
            )
        })
    }
}

/// Builder for creating output messages with fields.
pub struct MessageBuilder {
    level: Level,
    message: String,
    fields: HashMap<String, serde_json::Value>,
    source: Option<String>,
}

impl MessageBuilder {
    /// Create a new message builder.
    pub fn new(level: Level, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            fields: HashMap::new(),
            source: None,
        }
    }

    /// Add a field.
    pub fn field(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.fields.insert(key.into(), v);
        }
        self
    }

    /// Set source context.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Build the output message.
    pub fn build(self) -> OutputMessage {
        OutputMessage {
            timestamp: Utc::now(),
            level: self.level,
            message: self.message,
            fields: self.fields,
            source: self.source,
        }
    }
}
