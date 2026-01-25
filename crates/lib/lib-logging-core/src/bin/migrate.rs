//! Migration binary for logging database schema.

use lib_migrations_core::{MigrationEngine, Phase};
use lib_migrations_sql::SqlMigration;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let mut engine = MigrationEngine::new(pool);

    // Register migrations
    register_migrations(&mut engine)?;

    // Parse command
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("all");

    match command {
        "pre" => {
            println!("Running pre-deploy migrations...");
            engine.migrate_phase(Phase::PreDeploy).await?;
            println!("✓ Pre-deploy migrations complete");
        }
        "post" => {
            println!("Running post-deploy migrations...");
            engine.migrate_phase(Phase::PostDeploy).await?;
            println!("✓ Post-deploy migrations complete");
        }
        "all" => {
            println!("Running all migrations...");
            engine.migrate().await?;
            println!("✓ All migrations complete");
        }
        "status" => {
            let status = engine.status().await?;
            println!("Migration Status:");
            println!("  Applied: {}", status.applied);
            println!("  Pending: {}", status.pending);
            if status.pending > 0 {
                println!("\nPending migrations:");
                for migration in status.pending_migrations {
                    println!("  - {} ({})", migration.name, migration.version);
                }
            }
        }
        "dry-run" => {
            let plan = engine.dry_run().await?;
            println!(
                "Dry Run - Would execute {} migrations:",
                plan.migrations.len()
            );
            for migration in plan.migrations {
                println!("  - {} ({})", migration.name, migration.version);
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Usage: logging-migrate <pre|post|all|status|dry-run>");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn register_migrations(engine: &mut MigrationEngine) -> anyhow::Result<()> {
    // Migration 001: Create logs table with TimescaleDB hypertable
    engine.add(
        SqlMigration::from_file(
            "001_create_logs_table",
            "migrations/001_create_logs_table.sql",
        )?
        .phase(Phase::PreDeploy),
    );

    Ok(())
}
