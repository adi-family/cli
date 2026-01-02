use lib_analytics_core::EnrichedEvent;
use sqlx::{PgPool, Postgres, QueryBuilder};
use tracing::{error, info, warn};

/// Database writer for analytics events
pub struct EventWriter {
    db_pool: PgPool,
}

impl EventWriter {
    /// Create a new event writer
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Write a batch of events to the database
    pub async fn write_batch(&self, batch: &[EnrichedEvent]) -> Result<(), sqlx::Error> {
        let count = batch.len();
        if count == 0 {
            return Ok(());
        }

        let start = std::time::Instant::now();

        // Build bulk insert query
        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO analytics_events (timestamp, event_type, service, user_id, data) ",
        );

        query_builder.push_values(batch.iter(), |mut b, event| {
            let event_type = event.event.event_type();
            let service = event
                .event
                .service()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let user_id = event.event.user_id();

            // Serialize full event as JSONB
            let data = match serde_json::to_value(&event.event) {
                Ok(json) => json,
                Err(e) => {
                    warn!("Failed to serialize event: {}", e);
                    serde_json::json!({})
                }
            };

            b.push_bind(event.timestamp)
                .push_bind(event_type)
                .push_bind(service)
                .push_bind(user_id)
                .push_bind(data);
        });

        let query = query_builder.build();

        // Execute insert
        query.execute(&self.db_pool).await?;

        let duration = start.elapsed();
        info!("Wrote {} analytics events in {:?}", count, duration);

        Ok(())
    }
}
