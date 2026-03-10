use sqlx::PgPool;

/// Type alias — `PgPool` is already `Clone + Send + Sync`, no wrapper needed.
pub type Database = PgPool;
