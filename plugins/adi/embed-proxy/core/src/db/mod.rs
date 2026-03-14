pub mod keys;
pub mod models;
pub mod platform_keys;
pub mod tokens;
pub mod usage;

use sqlx::PgPool;

/// Database wrapper providing access to the connection pool.
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

pub use keys::*;
pub use models::*;
pub use platform_keys::*;
pub use tokens::*;
pub use usage::*;
