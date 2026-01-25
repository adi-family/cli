//! Trace context for distributed tracing.
//!
//! Implements hierarchical trace/span model:
//! - Trace ID: Unique identifier for the entire request chain (UUID v7)
//! - Span ID: Unique identifier for a single operation within the trace
//! - Parent Span ID: Links spans in a parent-child hierarchy
//!
//! Also supports business correlation IDs:
//! - Cocoon ID: Correlate all logs for a specific cocoon
//! - User ID: Correlate all logs for a specific user
//! - Session ID: Correlate logs within a WebSocket/WebRTC session
//! - Hive ID: Correlate logs for hive orchestration

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Business-level correlation IDs for cross-request log correlation.
///
/// These IDs allow querying all logs related to a specific entity,
/// regardless of which request chain generated them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationIds {
    /// Cocoon device ID - correlate all logs for a cocoon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cocoon_id: Option<String>,

    /// User ID - correlate all logs for a user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    /// Session ID - correlate logs within a session
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Hive ID - correlate logs for hive orchestration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hive_id: Option<String>,
}

impl CorrelationIds {
    /// Create empty correlation IDs.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set cocoon ID.
    pub fn with_cocoon(mut self, cocoon_id: impl Into<String>) -> Self {
        self.cocoon_id = Some(cocoon_id.into());
        self
    }

    /// Set user ID.
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set session ID.
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set hive ID.
    pub fn with_hive(mut self, hive_id: impl Into<String>) -> Self {
        self.hive_id = Some(hive_id.into());
        self
    }

    /// Check if any correlation ID is set.
    pub fn is_empty(&self) -> bool {
        self.cocoon_id.is_none()
            && self.user_id.is_none()
            && self.session_id.is_none()
            && self.hive_id.is_none()
    }
}

/// Trace context that propagates across services.
///
/// Each service creates a new span while preserving the trace ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// Unique identifier for the entire request chain.
    /// Generated once at the entry point and propagated to all downstream services.
    pub trace_id: Uuid,

    /// Current span ID for this operation.
    pub span_id: Uuid,

    /// Parent span ID (if this is a child span).
    pub parent_span_id: Option<Uuid>,

    /// Business-level correlation IDs for cross-request correlation.
    #[serde(default, skip_serializing_if = "CorrelationIds::is_empty")]
    pub correlation: CorrelationIds,
}

impl TraceContext {
    /// Create a new trace context (root span).
    ///
    /// Use this when starting a new request chain.
    pub fn new() -> Self {
        Self {
            trace_id: Uuid::now_v7(),
            span_id: Uuid::now_v7(),
            parent_span_id: None,
            correlation: CorrelationIds::default(),
        }
    }

    /// Create a new trace context with correlation IDs.
    ///
    /// Use this when you know the business context (cocoon, user, etc.).
    pub fn with_correlation(correlation: CorrelationIds) -> Self {
        Self {
            trace_id: Uuid::now_v7(),
            span_id: Uuid::now_v7(),
            parent_span_id: None,
            correlation,
        }
    }

    /// Create a child span within the same trace.
    ///
    /// Use this when making a call to another service or starting a new operation.
    /// Correlation IDs are preserved.
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id: Uuid::now_v7(),
            parent_span_id: Some(self.span_id),
            correlation: self.correlation.clone(),
        }
    }

    /// Create a context from incoming headers.
    ///
    /// If trace_id is provided, continues the trace. Otherwise creates a new one.
    pub fn from_headers(trace_id: Option<Uuid>, parent_span_id: Option<Uuid>) -> Self {
        match trace_id {
            Some(tid) => Self {
                trace_id: tid,
                span_id: Uuid::now_v7(),
                parent_span_id,
                correlation: CorrelationIds::default(),
            },
            None => Self::new(),
        }
    }

    /// Set cocoon ID for correlation.
    pub fn with_cocoon(mut self, cocoon_id: impl Into<String>) -> Self {
        self.correlation.cocoon_id = Some(cocoon_id.into());
        self
    }

    /// Set user ID for correlation.
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.correlation.user_id = Some(user_id.into());
        self
    }

    /// Set session ID for correlation.
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.correlation.session_id = Some(session_id.into());
        self
    }

    /// Set hive ID for correlation.
    pub fn with_hive(mut self, hive_id: impl Into<String>) -> Self {
        self.correlation.hive_id = Some(hive_id.into());
        self
    }

    /// Get headers to propagate to downstream services.
    pub fn to_headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = vec![
            (crate::TRACE_ID_HEADER, self.trace_id.to_string()),
            (crate::SPAN_ID_HEADER, self.span_id.to_string()),
        ];

        // Add correlation ID headers
        if let Some(ref cocoon_id) = self.correlation.cocoon_id {
            headers.push((crate::COCOON_ID_HEADER, cocoon_id.clone()));
        }
        if let Some(ref user_id) = self.correlation.user_id {
            headers.push((crate::USER_ID_HEADER, user_id.clone()));
        }
        if let Some(ref session_id) = self.correlation.session_id {
            headers.push((crate::SESSION_ID_HEADER, session_id.clone()));
        }
        if let Some(ref hive_id) = self.correlation.hive_id {
            headers.push((crate::HIVE_ID_HEADER, hive_id.clone()));
        }

        headers
    }

    /// Create headers for a child span to pass to downstream services.
    ///
    /// This passes the current span_id as the parent_span_id.
    pub fn child_headers(&self) -> Vec<(&'static str, String)> {
        let mut headers = vec![
            (crate::TRACE_ID_HEADER, self.trace_id.to_string()),
            (crate::PARENT_SPAN_ID_HEADER, self.span_id.to_string()),
        ];

        // Add correlation ID headers
        if let Some(ref cocoon_id) = self.correlation.cocoon_id {
            headers.push((crate::COCOON_ID_HEADER, cocoon_id.clone()));
        }
        if let Some(ref user_id) = self.correlation.user_id {
            headers.push((crate::USER_ID_HEADER, user_id.clone()));
        }
        if let Some(ref session_id) = self.correlation.session_id {
            headers.push((crate::SESSION_ID_HEADER, session_id.clone()));
        }
        if let Some(ref hive_id) = self.correlation.hive_id {
            headers.push((crate::HIVE_ID_HEADER, hive_id.clone()));
        }

        headers
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Lightweight span context for passing around.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SpanContext {
    pub trace_id: Uuid,
    pub span_id: Uuid,
}

impl From<&TraceContext> for SpanContext {
    fn from(ctx: &TraceContext) -> Self {
        Self {
            trace_id: ctx.trace_id,
            span_id: ctx.span_id,
        }
    }
}

impl From<TraceContext> for SpanContext {
    fn from(ctx: TraceContext) -> Self {
        Self {
            trace_id: ctx.trace_id,
            span_id: ctx.span_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_context() {
        let ctx = TraceContext::new();
        assert!(ctx.parent_span_id.is_none());
        assert_ne!(ctx.trace_id, ctx.span_id);
        assert!(ctx.correlation.is_empty());
    }

    #[test]
    fn test_child_span() {
        let parent = TraceContext::new()
            .with_cocoon("cocoon-123")
            .with_user("user-456");
        let child = parent.child();

        // Same trace
        assert_eq!(parent.trace_id, child.trace_id);
        // Different spans
        assert_ne!(parent.span_id, child.span_id);
        // Parent linked
        assert_eq!(child.parent_span_id, Some(parent.span_id));
        // Correlation IDs preserved
        assert_eq!(child.correlation.cocoon_id, Some("cocoon-123".to_string()));
        assert_eq!(child.correlation.user_id, Some("user-456".to_string()));
    }

    #[test]
    fn test_from_headers() {
        let trace_id = Uuid::now_v7();
        let parent_span = Uuid::now_v7();

        let ctx = TraceContext::from_headers(Some(trace_id), Some(parent_span));

        assert_eq!(ctx.trace_id, trace_id);
        assert_eq!(ctx.parent_span_id, Some(parent_span));
    }

    #[test]
    fn test_correlation_ids() {
        let ctx = TraceContext::new()
            .with_cocoon("cocoon-abc")
            .with_user("user-xyz")
            .with_session("session-123")
            .with_hive("hive-001");

        assert_eq!(ctx.correlation.cocoon_id, Some("cocoon-abc".to_string()));
        assert_eq!(ctx.correlation.user_id, Some("user-xyz".to_string()));
        assert_eq!(ctx.correlation.session_id, Some("session-123".to_string()));
        assert_eq!(ctx.correlation.hive_id, Some("hive-001".to_string()));
        assert!(!ctx.correlation.is_empty());
    }

    #[test]
    fn test_with_correlation() {
        let corr = CorrelationIds::new()
            .with_cocoon("cocoon-test")
            .with_user("user-test");

        let ctx = TraceContext::with_correlation(corr);

        assert_eq!(ctx.correlation.cocoon_id, Some("cocoon-test".to_string()));
        assert_eq!(ctx.correlation.user_id, Some("user-test".to_string()));
    }

    #[test]
    fn test_headers_include_correlation() {
        let ctx = TraceContext::new().with_cocoon("cocoon-xyz");

        let headers = ctx.to_headers();
        let cocoon_header = headers.iter().find(|(k, _)| *k == crate::COCOON_ID_HEADER);

        assert!(cocoon_header.is_some());
        assert_eq!(cocoon_header.unwrap().1, "cocoon-xyz");
    }
}
