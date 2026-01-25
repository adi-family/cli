//! Log writer - persists logs to TimescaleDB.

use lib_logging_core::EnrichedLogEntry;
use sqlx::PgPool;

/// Writes logs to the database.
#[derive(Clone)]
pub struct LogWriter {
    pool: PgPool,
}

impl LogWriter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Write a batch of logs to the database.
    pub async fn write_batch(&self, entries: &[EnrichedLogEntry]) -> Result<(), sqlx::Error> {
        if entries.is_empty() {
            return Ok(());
        }

        // Use a transaction for batch insert
        let mut tx = self.pool.begin().await?;

        for entry in entries {
            let level = entry.entry.level as i16;
            let level_name = entry.entry.level.as_str();

            let (error_kind, error_message, error_stack_trace) = match &entry.entry.error {
                Some(err) => (
                    Some(err.kind.as_str()),
                    Some(err.message.as_str()),
                    err.stack_trace.as_deref(),
                ),
                None => (None, None, None),
            };

            let fields = if entry.entry.fields.is_empty() {
                None
            } else {
                Some(serde_json::to_value(&entry.entry.fields).unwrap_or_default())
            };

            sqlx::query(
                r#"
                INSERT INTO logs (
                    timestamp, service, hostname, environment, version,
                    level, level_name, message,
                    trace_id, span_id, parent_span_id,
                    fields, error_kind, error_message, error_stack_trace,
                    source, target
                ) VALUES (
                    $1, $2, $3, $4, $5,
                    $6, $7, $8,
                    $9, $10, $11,
                    $12, $13, $14, $15,
                    $16, $17
                )
                "#,
            )
            .bind(entry.timestamp)
            .bind(&entry.service)
            .bind(&entry.hostname)
            .bind(&entry.environment)
            .bind(&entry.version)
            .bind(level)
            .bind(level_name)
            .bind(&entry.entry.message)
            .bind(entry.entry.trace_id)
            .bind(entry.entry.span_id)
            .bind(entry.entry.parent_span_id)
            .bind(fields)
            .bind(error_kind)
            .bind(error_message)
            .bind(error_stack_trace)
            .bind(&entry.entry.source)
            .bind(&entry.entry.target)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        tracing::debug!("Wrote {} log entries to database", entries.len());
        Ok(())
    }
}
