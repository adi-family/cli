//! Database migration binary for adi-api-proxy.

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is required");

    println!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Running migrations...");
    sqlx::migrate!("../core/migrations").run(&pool).await?;

    println!("All migrations complete!");
    Ok(())
}
