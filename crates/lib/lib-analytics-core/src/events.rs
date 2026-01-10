use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Analytics event types covering all ADI services
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnalyticsEvent {
    // ===== Authentication Events =====
    /// User requested login code
    AuthLoginAttempt {
        user_id: Option<Uuid>,
        email: String,
        success: bool,
        error: Option<String>,
    },

    /// User verified login code
    AuthCodeVerified {
        user_id: Uuid,
        success: bool,
        error: Option<String>,
    },

    /// Token refresh attempt
    AuthTokenRefresh {
        user_id: Uuid,
        success: bool,
        error: Option<String>,
    },

    /// Session validation
    AuthSessionValidated {
        user_id: Uuid,
        valid: bool,
    },

    // ===== Task Events =====
    /// Task created
    TaskCreated {
        task_id: Uuid,
        user_id: Uuid,
        project_id: Option<Uuid>,
        cocoon_id: Option<Uuid>,
        command: String,
    },

    /// Task started execution
    TaskStarted {
        task_id: Uuid,
        user_id: Uuid,
        cocoon_id: Option<Uuid>,
    },

    /// Task completed successfully
    TaskCompleted {
        task_id: Uuid,
        user_id: Uuid,
        duration_ms: i64,
        exit_code: i32,
    },

    /// Task failed
    TaskFailed {
        task_id: Uuid,
        user_id: Uuid,
        duration_ms: Option<i64>,
        exit_code: Option<i32>,
        error: String,
    },

    /// Task cancelled by user
    TaskCancelled {
        task_id: Uuid,
        user_id: Uuid,
        duration_ms: Option<i64>,
    },

    // ===== Integration Events =====
    /// Integration connected
    IntegrationConnected {
        integration_id: Uuid,
        user_id: Uuid,
        provider: String,
        project_id: Option<Uuid>,
    },

    /// Integration disconnected
    IntegrationDisconnected {
        integration_id: Uuid,
        user_id: Uuid,
        provider: String,
        reason: Option<String>,
    },

    /// Integration used
    IntegrationUsed {
        integration_id: Uuid,
        user_id: Uuid,
        provider: String,
        action: String,
    },

    /// Integration error occurred
    IntegrationError {
        integration_id: Uuid,
        user_id: Uuid,
        provider: String,
        error: String,
    },

    /// OAuth flow started
    OAuthFlowStarted {
        user_id: Uuid,
        provider: String,
        state: String,
    },

    /// OAuth flow completed
    OAuthFlowCompleted {
        user_id: Uuid,
        provider: String,
        success: bool,
        error: Option<String>,
    },

    // ===== Webhook Events =====
    /// Webhook received
    WebhookReceived {
        integration_id: Option<Uuid>,
        provider: String,
        event_type: String,
        delivery_id: String,
    },

    /// Webhook processing completed
    WebhookProcessed {
        integration_id: Option<Uuid>,
        provider: String,
        event_type: String,
        delivery_id: String,
        success: bool,
        duration_ms: i64,
        error: Option<String>,
    },

    // ===== Cocoon/Device Events =====
    /// Cocoon registered
    CocoonRegistered {
        cocoon_id: Uuid,
        user_id: Uuid,
        device_name: Option<String>,
    },

    /// Cocoon connected to signaling server
    CocoonConnected {
        cocoon_id: Uuid,
        user_id: Option<Uuid>,
    },

    /// Cocoon disconnected
    CocoonDisconnected {
        cocoon_id: Uuid,
        user_id: Option<Uuid>,
        duration_seconds: i64,
    },

    /// Cocoon claimed by user
    CocoonClaimed {
        cocoon_id: Uuid,
        user_id: Uuid,
        via_setup_token: bool,
    },

    /// Cocoon setup token created
    CocoonSetupTokenCreated {
        token_id: Uuid,
        user_id: Uuid,
        cocoon_name: Option<String>,
    },

    /// Cocoon setup token used
    CocoonSetupTokenUsed {
        token_id: Uuid,
        cocoon_id: Uuid,
        user_id: Uuid,
    },

    // ===== Project Events =====
    /// Project created
    ProjectCreated {
        project_id: Uuid,
        user_id: Uuid,
        name: String,
    },

    /// Project updated
    ProjectUpdated {
        project_id: Uuid,
        user_id: Uuid,
    },

    /// Project deleted
    ProjectDeleted {
        project_id: Uuid,
        user_id: Uuid,
    },

    // ===== API Request Events =====
    /// API request made
    ApiRequest {
        service: String,
        endpoint: String,
        method: String,
        status_code: u16,
        duration_ms: i64,
        user_id: Option<Uuid>,
    },

    // ===== Database Query Events =====
    /// Database query executed
    DatabaseQuery {
        service: String,
        query_type: String,
        duration_ms: i64,
        rows_affected: Option<i64>,
    },

    // ===== Error Events =====
    /// Application error occurred
    ApplicationError {
        service: String,
        error_type: String,
        error_message: String,
        user_id: Option<Uuid>,
        context: Option<serde_json::Value>,
    },
}

impl AnalyticsEvent {
    /// Get the event type as a string
    pub fn event_type(&self) -> &'static str {
        match self {
            AnalyticsEvent::AuthLoginAttempt { .. } => "auth_login_attempt",
            AnalyticsEvent::AuthCodeVerified { .. } => "auth_code_verified",
            AnalyticsEvent::AuthTokenRefresh { .. } => "auth_token_refresh",
            AnalyticsEvent::AuthSessionValidated { .. } => "auth_session_validated",
            AnalyticsEvent::TaskCreated { .. } => "task_created",
            AnalyticsEvent::TaskStarted { .. } => "task_started",
            AnalyticsEvent::TaskCompleted { .. } => "task_completed",
            AnalyticsEvent::TaskFailed { .. } => "task_failed",
            AnalyticsEvent::TaskCancelled { .. } => "task_cancelled",
            AnalyticsEvent::IntegrationConnected { .. } => "integration_connected",
            AnalyticsEvent::IntegrationDisconnected { .. } => "integration_disconnected",
            AnalyticsEvent::IntegrationUsed { .. } => "integration_used",
            AnalyticsEvent::IntegrationError { .. } => "integration_error",
            AnalyticsEvent::OAuthFlowStarted { .. } => "oauth_flow_started",
            AnalyticsEvent::OAuthFlowCompleted { .. } => "oauth_flow_completed",
            AnalyticsEvent::WebhookReceived { .. } => "webhook_received",
            AnalyticsEvent::WebhookProcessed { .. } => "webhook_processed",
            AnalyticsEvent::CocoonRegistered { .. } => "cocoon_registered",
            AnalyticsEvent::CocoonConnected { .. } => "cocoon_connected",
            AnalyticsEvent::CocoonDisconnected { .. } => "cocoon_disconnected",
            AnalyticsEvent::CocoonClaimed { .. } => "cocoon_claimed",
            AnalyticsEvent::CocoonSetupTokenCreated { .. } => "cocoon_setup_token_created",
            AnalyticsEvent::CocoonSetupTokenUsed { .. } => "cocoon_setup_token_used",
            AnalyticsEvent::ProjectCreated { .. } => "project_created",
            AnalyticsEvent::ProjectUpdated { .. } => "project_updated",
            AnalyticsEvent::ProjectDeleted { .. } => "project_deleted",
            AnalyticsEvent::ApiRequest { .. } => "api_request",
            AnalyticsEvent::DatabaseQuery { .. } => "database_query",
            AnalyticsEvent::ApplicationError { .. } => "application_error",
        }
    }

    /// Get the service that generated this event
    pub fn service(&self) -> Option<&str> {
        match self {
            AnalyticsEvent::ApiRequest { service, .. } => Some(service),
            AnalyticsEvent::DatabaseQuery { service, .. } => Some(service),
            AnalyticsEvent::ApplicationError { service, .. } => Some(service),
            _ => None,
        }
    }

    /// Get the user ID if available
    pub fn user_id(&self) -> Option<Uuid> {
        match self {
            AnalyticsEvent::AuthLoginAttempt { user_id, .. } => *user_id,
            AnalyticsEvent::AuthCodeVerified { user_id, .. } => Some(*user_id),
            AnalyticsEvent::AuthTokenRefresh { user_id, .. } => Some(*user_id),
            AnalyticsEvent::AuthSessionValidated { user_id, .. } => Some(*user_id),
            AnalyticsEvent::TaskCreated { user_id, .. } => Some(*user_id),
            AnalyticsEvent::TaskStarted { user_id, .. } => Some(*user_id),
            AnalyticsEvent::TaskCompleted { user_id, .. } => Some(*user_id),
            AnalyticsEvent::TaskFailed { user_id, .. } => Some(*user_id),
            AnalyticsEvent::TaskCancelled { user_id, .. } => Some(*user_id),
            AnalyticsEvent::IntegrationConnected { user_id, .. } => Some(*user_id),
            AnalyticsEvent::IntegrationDisconnected { user_id, .. } => Some(*user_id),
            AnalyticsEvent::IntegrationUsed { user_id, .. } => Some(*user_id),
            AnalyticsEvent::IntegrationError { user_id, .. } => Some(*user_id),
            AnalyticsEvent::OAuthFlowStarted { user_id, .. } => Some(*user_id),
            AnalyticsEvent::OAuthFlowCompleted { user_id, .. } => Some(*user_id),
            AnalyticsEvent::CocoonRegistered { user_id, .. } => Some(*user_id),
            AnalyticsEvent::CocoonConnected { user_id, .. } => *user_id,
            AnalyticsEvent::CocoonDisconnected { user_id, .. } => *user_id,
            AnalyticsEvent::CocoonClaimed { user_id, .. } => Some(*user_id),
            AnalyticsEvent::CocoonSetupTokenCreated { user_id, .. } => Some(*user_id),
            AnalyticsEvent::CocoonSetupTokenUsed { user_id, .. } => Some(*user_id),
            AnalyticsEvent::ProjectCreated { user_id, .. } => Some(*user_id),
            AnalyticsEvent::ProjectUpdated { user_id, .. } => Some(*user_id),
            AnalyticsEvent::ProjectDeleted { user_id, .. } => Some(*user_id),
            AnalyticsEvent::ApiRequest { user_id, .. } => *user_id,
            AnalyticsEvent::ApplicationError { user_id, .. } => *user_id,
            _ => None,
        }
    }
}

/// Enriched event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedEvent {
    pub timestamp: DateTime<Utc>,
    pub event: AnalyticsEvent,
    pub hostname: Option<String>,
    pub environment: Option<String>,
}

impl EnrichedEvent {
    pub fn new(event: AnalyticsEvent) -> Self {
        Self {
            timestamp: Utc::now(),
            event,
            hostname: std::env::var("HOSTNAME").ok(),
            environment: std::env::var("ENVIRONMENT").ok(),
        }
    }
}
