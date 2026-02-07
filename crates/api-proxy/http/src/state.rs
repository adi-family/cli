//! Application state for the HTTP server.

use api_proxy_core::transform::TransformEngine;
use api_proxy_core::{Config, Database, SecretManager};
use lib_analytics_core::AnalyticsClient;
use std::sync::Arc;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool.
    pub db: Database,
    /// Service configuration.
    pub config: Arc<Config>,
    /// Analytics client for event tracking.
    pub analytics: AnalyticsClient,
    /// Secret manager for encryption/decryption.
    pub secrets: SecretManager,
    /// Rhai transform engine.
    pub transform: TransformEngine,
}

impl AppState {
    /// Create a new application state.
    pub fn new(
        db: Database,
        config: Config,
        analytics: AnalyticsClient,
        secrets: SecretManager,
    ) -> Self {
        Self {
            db,
            config: Arc::new(config),
            analytics,
            secrets,
            transform: TransformEngine::new(),
        }
    }
}
