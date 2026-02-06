// Copyright (c) 2024-2025 Ihor
// SPDX-License-Identifier: BSL-1.1
// See LICENSE file for details

//! Progress indicators for console output.
//!
//! Supports three modes:
//! - Interactive terminal: Replace lines with spinners/progress bars
//! - Non-interactive: Print progress updates as separate lines
//! - JSON stream: Send structured progress events

use crate::{console as out_console, theme, OutputMode};
use chrono::{DateTime, Utc};
use console::{StyledObject, Term};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Default spinner tick interval.
const SPINNER_INTERVAL: Duration = Duration::from_millis(80);

/// Progress event types for JSON stream mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProgressEvent {
    /// Progress started.
    Start {
        id: String,
        message: String,
        total: Option<u64>,
        timestamp: DateTime<Utc>,
    },
    /// Progress updated.
    Update {
        id: String,
        message: Option<String>,
        current: u64,
        total: Option<u64>,
        percent: Option<f32>,
        timestamp: DateTime<Utc>,
    },
    /// Progress completed successfully.
    Complete {
        id: String,
        message: Option<String>,
        timestamp: DateTime<Utc>,
    },
    /// Progress failed.
    Fail {
        id: String,
        message: Option<String>,
        error: Option<String>,
        timestamp: DateTime<Utc>,
    },
}

impl ProgressEvent {
    /// Serialize to JSON line.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"type":"error"}"#.to_string())
    }
}

/// Check if the terminal is interactive (supports cursor movement).
pub fn is_interactive() -> bool {
    Term::stdout().is_term()
}

/// Spinner for indeterminate progress.
pub struct Spinner {
    id: String,
    message: String,
    term: Term,
    running: Arc<AtomicBool>,
    frame: usize,
    last_tick: Instant,
    interactive: bool,
    mode: OutputMode,
}

impl Spinner {
    /// Create a new spinner with a message.
    pub fn new(message: impl Into<String>) -> Self {
        let mode = out_console().mode();
        let interactive = is_interactive() && mode.is_text();

        Self {
            id: uuid_simple(),
            message: message.into(),
            term: Term::stdout(),
            running: Arc::new(AtomicBool::new(false)),
            frame: 0,
            last_tick: Instant::now(),
            interactive,
            mode,
        }
    }

    /// Start the spinner.
    pub fn start(&mut self) {
        self.running.store(true, Ordering::SeqCst);

        match self.mode {
            OutputMode::JsonStream => {
                let event = ProgressEvent::Start {
                    id: self.id.clone(),
                    message: self.message.clone(),
                    total: None,
                    timestamp: Utc::now(),
                };
                println!("{}", event.to_json());
            }
            OutputMode::Text if self.interactive => {
                self.render();
            }
            OutputMode::Text => {
                println!("... {}", self.message);
            }
        }
    }

    /// Update the spinner message.
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();

        if self.mode.is_json_stream() {
            let event = ProgressEvent::Update {
                id: self.id.clone(),
                message: Some(self.message.clone()),
                current: 0,
                total: None,
                percent: None,
                timestamp: Utc::now(),
            };
            println!("{}", event.to_json());
        } else if self.interactive {
            self.render();
        }
    }

    /// Tick the spinner animation (call in a loop or timer).
    pub fn tick(&mut self) {
        if !self.running.load(Ordering::SeqCst) {
            return;
        }

        if self.last_tick.elapsed() >= SPINNER_INTERVAL {
            self.frame = (self.frame + 1) % theme::SPINNER_FRAMES.len();
            self.last_tick = Instant::now();

            if self.interactive && self.mode.is_text() {
                self.render();
            }
        }
    }

    /// Render the current spinner state.
    fn render(&self) {
        let frame = theme::SPINNER_FRAMES[self.frame];
        let line = format!("{} {}", theme::brand(frame), self.message);
        let _ = self.term.clear_line();
        let _ = write!(&self.term, "\r{}", line);
        let _ = self.term.flush();
    }

    /// Complete the spinner with success.
    pub fn success(self, message: Option<&str>) {
        self.finish(true, message, None);
    }

    /// Complete the spinner with failure.
    pub fn fail(self, message: Option<&str>, error: Option<&str>) {
        self.finish(false, message, error);
    }

    /// Finish the spinner.
    fn finish(self, success: bool, message: Option<&str>, error: Option<&str>) {
        let final_message = message.unwrap_or(&self.message);

        match self.mode {
            OutputMode::JsonStream => {
                let event = if success {
                    ProgressEvent::Complete {
                        id: self.id,
                        message: Some(final_message.to_string()),
                        timestamp: Utc::now(),
                    }
                } else {
                    ProgressEvent::Fail {
                        id: self.id,
                        message: Some(final_message.to_string()),
                        error: error.map(String::from),
                        timestamp: Utc::now(),
                    }
                };
                println!("{}", event.to_json());
            }
            OutputMode::Text if self.interactive => {
                let _ = self.term.clear_line();
                if success {
                    println!(
                        "\r{} {}",
                        theme::success(theme::icons::SUCCESS),
                        final_message
                    );
                } else {
                    eprintln!(
                        "\r{} {}{}",
                        theme::error(theme::icons::ERROR),
                        final_message,
                        error.map(|e| format!(": {}", e)).unwrap_or_default()
                    );
                }
            }
            OutputMode::Text => {
                if success {
                    println!(
                        "{} {}",
                        theme::success(theme::icons::SUCCESS),
                        final_message
                    );
                } else {
                    eprintln!(
                        "{} {}{}",
                        theme::error(theme::icons::ERROR),
                        final_message,
                        error.map(|e| format!(": {}", e)).unwrap_or_default()
                    );
                }
            }
        }
    }
}

/// Progress bar for determinate progress.
pub struct ProgressBar {
    id: String,
    message: String,
    current: Arc<AtomicU64>,
    total: u64,
    term: Term,
    interactive: bool,
    mode: OutputMode,
    width: usize,
    last_percent: u8,
}

impl ProgressBar {
    /// Create a new progress bar.
    pub fn new(total: u64, message: impl Into<String>) -> Self {
        let mode = out_console().mode();
        let interactive = is_interactive() && mode.is_text();

        Self {
            id: uuid_simple(),
            message: message.into(),
            current: Arc::new(AtomicU64::new(0)),
            total,
            term: Term::stdout(),
            interactive,
            mode,
            width: 30,
            last_percent: 0,
        }
    }

    /// Start the progress bar.
    pub fn start(&self) {
        match self.mode {
            OutputMode::JsonStream => {
                let event = ProgressEvent::Start {
                    id: self.id.clone(),
                    message: self.message.clone(),
                    total: Some(self.total),
                    timestamp: Utc::now(),
                };
                println!("{}", event.to_json());
            }
            OutputMode::Text if self.interactive => {
                self.render();
            }
            OutputMode::Text => {
                println!("{} (0/{})", self.message, self.total);
            }
        }
    }

    /// Set the current progress value.
    pub fn set(&mut self, value: u64) {
        self.current.store(value.min(self.total), Ordering::SeqCst);
        self.update_display();
    }

    /// Increment progress by 1.
    pub fn inc(&mut self) {
        self.inc_by(1);
    }

    /// Increment progress by a specific amount.
    pub fn inc_by(&mut self, delta: u64) {
        let new_val = self
            .current
            .fetch_add(delta, Ordering::SeqCst)
            .saturating_add(delta)
            .min(self.total);
        self.current.store(new_val, Ordering::SeqCst);
        self.update_display();
    }

    /// Get the current progress value.
    pub fn position(&self) -> u64 {
        self.current.load(Ordering::SeqCst)
    }

    /// Get the progress percentage (0.0 - 100.0).
    pub fn percent(&self) -> f32 {
        if self.total == 0 {
            return 100.0;
        }
        (self.position() as f32 / self.total as f32) * 100.0
    }

    /// Update the message.
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.update_display();
    }

    /// Update the display based on current state.
    fn update_display(&mut self) {
        let current = self.position();
        let percent = self.percent() as u8;

        match self.mode {
            OutputMode::JsonStream => {
                let event = ProgressEvent::Update {
                    id: self.id.clone(),
                    message: Some(self.message.clone()),
                    current,
                    total: Some(self.total),
                    percent: Some(self.percent()),
                    timestamp: Utc::now(),
                };
                println!("{}", event.to_json());
            }
            OutputMode::Text if self.interactive => {
                self.render();
            }
            OutputMode::Text => {
                // Only print on significant changes (every 10%)
                if percent / 10 != self.last_percent / 10 || current == self.total {
                    println!("{} [{}/{}] {}%", self.message, current, self.total, percent);
                    self.last_percent = percent;
                }
            }
        }
    }

    /// Render the progress bar.
    fn render(&self) {
        let current = self.position();
        let percent = self.percent();
        let filled = ((percent / 100.0) * self.width as f32) as usize;
        let empty = self.width.saturating_sub(filled);

        let bar = format!(
            "{}{}",
            theme::icons::BAR_FILLED.repeat(filled),
            theme::icons::BAR_EMPTY.repeat(empty)
        );

        let line = format!(
            "{} {} [{}/{}] {:.0}%",
            self.message,
            theme::brand(&bar),
            current,
            self.total,
            percent
        );

        let _ = self.term.clear_line();
        let _ = write!(&self.term, "\r{}", line);
        let _ = self.term.flush();
    }

    /// Complete the progress bar with success.
    pub fn success(self, message: Option<&str>) {
        self.finish(true, message);
    }

    /// Complete the progress bar with failure.
    pub fn fail(self, message: Option<&str>) {
        self.finish(false, message);
    }

    /// Finish the progress bar.
    fn finish(self, success: bool, message: Option<&str>) {
        let final_message = message.unwrap_or(&self.message);

        match self.mode {
            OutputMode::JsonStream => {
                let event = if success {
                    ProgressEvent::Complete {
                        id: self.id,
                        message: Some(final_message.to_string()),
                        timestamp: Utc::now(),
                    }
                } else {
                    ProgressEvent::Fail {
                        id: self.id,
                        message: Some(final_message.to_string()),
                        error: None,
                        timestamp: Utc::now(),
                    }
                };
                println!("{}", event.to_json());
            }
            OutputMode::Text if self.interactive => {
                let _ = self.term.clear_line();
                if success {
                    println!(
                        "\r{} {}",
                        theme::success(theme::icons::SUCCESS),
                        final_message
                    );
                } else {
                    eprintln!(
                        "\r{} {}",
                        theme::error(theme::icons::ERROR),
                        final_message
                    );
                }
            }
            OutputMode::Text => {
                if success {
                    println!(
                        "{} {}",
                        theme::success(theme::icons::SUCCESS),
                        final_message
                    );
                } else {
                    eprintln!(
                        "{} {}",
                        theme::error(theme::icons::ERROR),
                        final_message
                    );
                }
            }
        }
    }
}

/// Step counter for multi-step operations.
pub struct StepProgress {
    id: String,
    current: usize,
    total: usize,
    message: String,
    term: Term,
    interactive: bool,
    mode: OutputMode,
}

impl StepProgress {
    /// Create a new step counter.
    pub fn new(total: usize, message: impl Into<String>) -> Self {
        let mode = out_console().mode();
        let interactive = is_interactive() && mode.is_text();

        Self {
            id: uuid_simple(),
            current: 0,
            total,
            message: message.into(),
            term: Term::stdout(),
            interactive,
            mode,
        }
    }

    /// Start the step progress.
    pub fn start(&self) {
        match self.mode {
            OutputMode::JsonStream => {
                let event = ProgressEvent::Start {
                    id: self.id.clone(),
                    message: self.message.clone(),
                    total: Some(self.total as u64),
                    timestamp: Utc::now(),
                };
                println!("{}", event.to_json());
            }
            OutputMode::Text => {
                let prefix = format!("[{}/{}]", self.current, self.total);
                println!("{} {}", theme::muted(prefix), self.message);
            }
        }
    }

    /// Move to the next step with a new message.
    pub fn next_step(&mut self, message: impl Into<String>) {
        self.current = (self.current + 1).min(self.total);
        self.message = message.into();

        match self.mode {
            OutputMode::JsonStream => {
                let event = ProgressEvent::Update {
                    id: self.id.clone(),
                    message: Some(self.message.clone()),
                    current: self.current as u64,
                    total: Some(self.total as u64),
                    percent: Some((self.current as f32 / self.total as f32) * 100.0),
                    timestamp: Utc::now(),
                };
                println!("{}", event.to_json());
            }
            OutputMode::Text if self.interactive => {
                let prefix = format!("[{}/{}]", self.current, self.total);
                let _ = self.term.clear_line();
                let _ = writeln!(&self.term, "\r{} {}", theme::brand(prefix), self.message);
            }
            OutputMode::Text => {
                let prefix = format!("[{}/{}]", self.current, self.total);
                println!("{} {}", theme::brand(prefix), self.message);
            }
        }
    }

    /// Get current step number.
    pub fn current(&self) -> usize {
        self.current
    }

    /// Complete all steps with success.
    pub fn success(self, message: Option<&str>) {
        let final_message = message.unwrap_or("Complete");

        match self.mode {
            OutputMode::JsonStream => {
                let event = ProgressEvent::Complete {
                    id: self.id,
                    message: Some(final_message.to_string()),
                    timestamp: Utc::now(),
                };
                println!("{}", event.to_json());
            }
            OutputMode::Text => {
                println!(
                    "{} {}",
                    theme::success(theme::icons::SUCCESS),
                    final_message
                );
            }
        }
    }

    /// Fail at current step.
    pub fn fail(self, message: Option<&str>) {
        let final_message = message.unwrap_or("Failed");

        match self.mode {
            OutputMode::JsonStream => {
                let event = ProgressEvent::Fail {
                    id: self.id,
                    message: Some(final_message.to_string()),
                    error: None,
                    timestamp: Utc::now(),
                };
                println!("{}", event.to_json());
            }
            OutputMode::Text => {
                eprintln!(
                    "{} {} at step {}/{}",
                    theme::error(theme::icons::ERROR),
                    final_message,
                    self.current,
                    self.total
                );
            }
        }
    }
}

/// Multi-line progress tracker for parallel operations.
pub struct MultiProgress {
    #[allow(dead_code)]
    id: String,
    items: Vec<MultiProgressItem>,
    #[allow(dead_code)]
    term: Term,
    #[allow(dead_code)]
    interactive: bool,
    mode: OutputMode,
}

/// An item in a multi-progress tracker.
#[derive(Clone)]
struct MultiProgressItem {
    id: String,
    message: String,
    status: MultiProgressStatus,
}

/// Status of a multi-progress item.
#[derive(Clone, Copy, PartialEq)]
enum MultiProgressStatus {
    Pending,
    InProgress,
    Complete,
    Failed,
}

impl MultiProgress {
    /// Create a new multi-progress tracker.
    pub fn new() -> Self {
        let mode = out_console().mode();
        let interactive = is_interactive() && mode.is_text();

        Self {
            id: uuid_simple(),
            items: Vec::new(),
            term: Term::stdout(),
            interactive,
            mode,
        }
    }

    /// Add an item to track.
    pub fn add(&mut self, message: impl Into<String>) -> usize {
        let item = MultiProgressItem {
            id: uuid_simple(),
            message: message.into(),
            status: MultiProgressStatus::Pending,
        };
        self.items.push(item);
        self.items.len() - 1
    }

    /// Start tracking an item.
    pub fn start_item(&mut self, index: usize) {
        if let Some(item) = self.items.get_mut(index) {
            item.status = MultiProgressStatus::InProgress;
            self.render_item(index);
        }
    }

    /// Complete an item successfully.
    pub fn complete_item(&mut self, index: usize, message: Option<&str>) {
        if let Some(item) = self.items.get_mut(index) {
            if let Some(msg) = message {
                item.message = msg.to_string();
            }
            item.status = MultiProgressStatus::Complete;
            self.render_item(index);
        }
    }

    /// Fail an item.
    pub fn fail_item(&mut self, index: usize, message: Option<&str>) {
        if let Some(item) = self.items.get_mut(index) {
            if let Some(msg) = message {
                item.message = msg.to_string();
            }
            item.status = MultiProgressStatus::Failed;
            self.render_item(index);
        }
    }

    /// Render a single item.
    fn render_item(&self, index: usize) {
        let Some(item) = self.items.get(index) else {
            return;
        };

        match self.mode {
            OutputMode::JsonStream => {
                let event = match item.status {
                    MultiProgressStatus::Pending => ProgressEvent::Start {
                        id: item.id.clone(),
                        message: item.message.clone(),
                        total: None,
                        timestamp: Utc::now(),
                    },
                    MultiProgressStatus::InProgress => ProgressEvent::Update {
                        id: item.id.clone(),
                        message: Some(item.message.clone()),
                        current: 0,
                        total: None,
                        percent: None,
                        timestamp: Utc::now(),
                    },
                    MultiProgressStatus::Complete => ProgressEvent::Complete {
                        id: item.id.clone(),
                        message: Some(item.message.clone()),
                        timestamp: Utc::now(),
                    },
                    MultiProgressStatus::Failed => ProgressEvent::Fail {
                        id: item.id.clone(),
                        message: Some(item.message.clone()),
                        error: None,
                        timestamp: Utc::now(),
                    },
                };
                println!("{}", event.to_json());
            }
            OutputMode::Text => {
                let (icon, style_fn): (&str, fn(&str) -> StyledObject<&str>) = match item.status {
                    MultiProgressStatus::Pending => {
                        (theme::icons::PENDING, |s| theme::muted(s))
                    }
                    MultiProgressStatus::InProgress => {
                        (theme::icons::IN_PROGRESS, |s| theme::brand(s))
                    }
                    MultiProgressStatus::Complete => {
                        (theme::icons::SUCCESS, |s| theme::success(s))
                    }
                    MultiProgressStatus::Failed => {
                        (theme::icons::ERROR, |s| theme::error(s))
                    }
                };

                println!("{} {}", style_fn(icon), item.message);
            }
        }
    }

    /// Check if all items are complete (success or failure).
    pub fn is_complete(&self) -> bool {
        self.items.iter().all(|item| {
            matches!(
                item.status,
                MultiProgressStatus::Complete | MultiProgressStatus::Failed
            )
        })
    }

    /// Get count of completed items.
    pub fn completed_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.status == MultiProgressStatus::Complete)
            .count()
    }

    /// Get count of failed items.
    pub fn failed_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.status == MultiProgressStatus::Failed)
            .count()
    }
}

impl Default for MultiProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a simple unique ID.
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    format!("{:x}{:x}", now.as_nanos(), std::process::id())
}

// Convenience functions

/// Create and start a spinner.
pub fn spinner(message: impl Into<String>) -> Spinner {
    let mut s = Spinner::new(message);
    s.start();
    s
}

/// Create and start a progress bar.
pub fn progress_bar(total: u64, message: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new(total, message);
    pb.start();
    pb
}

/// Create and start a step progress.
pub fn steps(total: usize, message: impl Into<String>) -> StepProgress {
    let sp = StepProgress::new(total, message);
    sp.start();
    sp
}
