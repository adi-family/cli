//! Trace context for distributed tracing.
//!
//! Implements hierarchical trace/span model:
//! - Trace ID: Unique identifier for the entire request chain (UUID v7)
//! - Span ID: Unique identifier for a single operation within the trace
//! - Parent Span ID: Links spans in a parent-child hierarchy

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        }
    }

    /// Create a child span within the same trace.
    ///
    /// Use this when making a call to another service or starting a new operation.
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id: Uuid::now_v7(),
            parent_span_id: Some(self.span_id),
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
            },
            None => Self::new(),
        }
    }

    /// Get headers to propagate to downstream services.
    pub fn to_headers(&self) -> Vec<(&'static str, String)> {
        vec![
            (crate::TRACE_ID_HEADER, self.trace_id.to_string()),
            (crate::SPAN_ID_HEADER, self.span_id.to_string()),
        ]
    }

    /// Create headers for a child span to pass to downstream services.
    ///
    /// This passes the current span_id as the parent_span_id.
    pub fn child_headers(&self) -> Vec<(&'static str, String)> {
        vec![
            (crate::TRACE_ID_HEADER, self.trace_id.to_string()),
            (crate::PARENT_SPAN_ID_HEADER, self.span_id.to_string()),
        ]
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
    }

    #[test]
    fn test_child_span() {
        let parent = TraceContext::new();
        let child = parent.child();

        // Same trace
        assert_eq!(parent.trace_id, child.trace_id);
        // Different spans
        assert_ne!(parent.span_id, child.span_id);
        // Parent linked
        assert_eq!(child.parent_span_id, Some(parent.span_id));
    }

    #[test]
    fn test_from_headers() {
        let trace_id = Uuid::now_v7();
        let parent_span = Uuid::now_v7();

        let ctx = TraceContext::from_headers(Some(trace_id), Some(parent_span));

        assert_eq!(ctx.trace_id, trace_id);
        assert_eq!(ctx.parent_span_id, Some(parent_span));
    }
}
