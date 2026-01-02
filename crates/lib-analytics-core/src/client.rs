use crate::events::{AnalyticsEvent, EnrichedEvent};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Client for tracking analytics events
///
/// Sends events to the analytics ingestion service via HTTP.
/// All track() calls are non-blocking.
#[derive(Clone)]
pub struct AnalyticsClient {
    http_client: reqwest::Client,
    analytics_url: Arc<str>,
    sender: mpsc::UnboundedSender<EnrichedEvent>,
}

impl AnalyticsClient {
    /// Create a new analytics client
    ///
    /// # Arguments
    /// * `analytics_url` - Base URL of analytics ingestion service (e.g., "http://localhost:8094")
    ///
    /// Events are batched and sent asynchronously in the background.
    pub fn new(analytics_url: impl Into<String>) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let analytics_url: Arc<str> = analytics_url.into().into();
        let http_client = reqwest::Client::new();

        // Spawn background sender task
        let url = analytics_url.clone();
        let client = http_client.clone();
        tokio::spawn(async move {
            Self::send_loop(receiver, client, url).await;
        });

        Self {
            http_client,
            analytics_url,
            sender,
        }
    }

    /// Track an analytics event
    ///
    /// This is non-blocking and will not fail even if the service is unavailable.
    /// Events are enriched with timestamp and metadata before sending.
    pub fn track(&self, event: AnalyticsEvent) {
        let enriched = EnrichedEvent::new(event);
        // Ignore send errors (background task might be shut down)
        let _ = self.sender.send(enriched);
    }

    /// Track an event only if a condition is true
    pub fn track_if(&self, condition: bool, event: AnalyticsEvent) {
        if condition {
            self.track(event);
        }
    }

    /// Create a no-op client for testing or disabled analytics
    pub fn noop() -> Self {
        Self::new("http://localhost:9999")
    }

    /// Background task that batches and sends events
    async fn send_loop(
        mut receiver: mpsc::UnboundedReceiver<EnrichedEvent>,
        client: reqwest::Client,
        analytics_url: Arc<str>,
    ) {
        let mut batch = Vec::with_capacity(100);
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

        // Skip first tick (happens immediately)
        interval.tick().await;

        loop {
            tokio::select! {
                // Receive event
                Some(event) = receiver.recv() => {
                    batch.push(event);

                    // Send if batch is full
                    if batch.len() >= 100 {
                        Self::send_batch(&client, &analytics_url, &mut batch).await;
                    }
                }

                // Periodic flush
                _ = interval.tick() => {
                    if !batch.is_empty() {
                        Self::send_batch(&client, &analytics_url, &mut batch).await;
                    }
                }
            }
        }
    }

    /// Send a batch of events to the analytics service
    async fn send_batch(
        client: &reqwest::Client,
        analytics_url: &str,
        batch: &mut Vec<EnrichedEvent>,
    ) {
        let count = batch.len();
        if count == 0 {
            return;
        }

        let url = format!("{}/events/batch", analytics_url);

        match client.post(&url).json(&batch).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    tracing::debug!("Sent {} analytics events", count);
                } else {
                    tracing::warn!(
                        "Failed to send analytics events: HTTP {}",
                        response.status()
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Failed to send analytics events: {}", e);
            }
        }

        batch.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_client_creation() {
        let client = AnalyticsClient::new("http://localhost:8094");

        // Should not panic
        client.track(AnalyticsEvent::AuthLoginAttempt {
            user_id: Some(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            success: true,
            error: None,
        });
    }

    #[test]
    fn test_noop_client() {
        let client = AnalyticsClient::noop();

        // Should not panic
        client.track(AnalyticsEvent::AuthLoginAttempt {
            user_id: Some(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            success: true,
            error: None,
        });
    }
}
