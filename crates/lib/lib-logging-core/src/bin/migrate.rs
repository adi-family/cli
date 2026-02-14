//! Migration binary for logging database schema.
//!
//! Run migrations directly using psql since lib-migrations API changed.

use sqlx::postgres::PgPoolOptions;

const MIGRATION_001: &str = include_str!("../../migrations/001_create_logs_table.sql");
const MIGRATION_002: &str = include_str!("../../migrations/002_add_correlation_ids.sql");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();

    let database_url = lib_logging_core::env::database_url()
        .expect("DATABASE_URL must be set");

    println!("Connecting to database...");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Parse command
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("all");

    match command {
        "all" => {
            println!("Running all migrations...");
            
            // Migration 001: Create logs table
            println!("  Applying 001_create_logs_table...");
            match sqlx::raw_sql(MIGRATION_001).execute(&pool).await {
                Ok(_) => println!("    ✓ 001_create_logs_table applied"),
                Err(e) => {
                    // Check if it's "already exists" error
                    let err_str = e.to_string();
                    if err_str.contains("already exists") || err_str.contains("duplicate") {
                        println!("    ✓ 001_create_logs_table already applied (skipped)");
                    } else {
                        println!("    ✗ 001_create_logs_table failed: {}", e);
                    }
                }
            }

            // Migration 002: Add correlation IDs
            println!("  Applying 002_add_correlation_ids...");
            match sqlx::raw_sql(MIGRATION_002).execute(&pool).await {
                Ok(_) => println!("    ✓ 002_add_correlation_ids applied"),
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("already exists") || err_str.contains("duplicate") {
                        println!("    ✓ 002_add_correlation_ids already applied (skipped)");
                    } else {
                        println!("    ✗ 002_add_correlation_ids failed: {}", e);
                    }
                }
            }

            println!("\n✓ All migrations complete");
        }
        "status" => {
            println!("Checking migration status...");
            
            // Check if logs table exists
            let table_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'logs')"
            )
            .fetch_one(&pool)
            .await?;

            if table_exists {
                println!("  ✓ logs table exists");
                
                // Check if correlation columns exist
                let cocoon_col_exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT FROM information_schema.columns WHERE table_name = 'logs' AND column_name = 'cocoon_id')"
                )
                .fetch_one(&pool)
                .await?;

                if cocoon_col_exists {
                    println!("  ✓ correlation ID columns exist");
                } else {
                    println!("  ✗ correlation ID columns missing (run 'all' to apply)");
                }
            } else {
                println!("  ✗ logs table does not exist (run 'all' to apply)");
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Usage: logging-migrate <all|status>");
            std::process::exit(1);
        }
    }

    Ok(())
}
