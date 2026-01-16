use std::env;

use lib_migrations_sql::SqlMigration;
use sqlx::postgres::PgPoolOptions;

fn migrations() -> Vec<SqlMigration> {
    vec![
        SqlMigration::new(
            1,
            "001_create_balances",
            include_str!("../../migrations/001_create_balances.sql"),
        ),
        SqlMigration::new(
            2,
            "002_create_transactions",
            include_str!("../../migrations/002_create_transactions.sql"),
        ),
    ]
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is required");

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .map_err(|e| {
            let safe_url = redact_password(&database_url);
            anyhow::anyhow!(
                "Failed to connect to database at '{}': {}",
                safe_url,
                e
            )
        })?;

    sqlx::raw_sql(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            version BIGINT PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(&pool)
    .await?;

    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("all");

    let migrations = migrations();

    match command {
        "all" => {
            println!("Running all migrations...");
            for migration in &migrations {
                let applied: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM _migrations WHERE version = $1)",
                )
                .bind(migration.version() as i64)
                .fetch_one(&pool)
                .await?;

                if applied {
                    println!("  [skip] {} (already applied)", migration.name());
                    continue;
                }

                println!("  [run]  {}...", migration.name());

                sqlx::raw_sql(migration.up_sql()).execute(&pool).await?;

                sqlx::query("INSERT INTO _migrations (version, name) VALUES ($1, $2)")
                    .bind(migration.version() as i64)
                    .bind(migration.name())
                    .execute(&pool)
                    .await?;

                println!("         done");
            }
            println!("All migrations complete.");
        }
        "status" => {
            println!("Migration status:");
            for migration in &migrations {
                let applied: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM _migrations WHERE version = $1)",
                )
                .bind(migration.version() as i64)
                .fetch_one(&pool)
                .await?;

                let status = if applied { "applied" } else { "pending" };
                println!("  {}: {}", migration.name(), status);
            }
        }
        "dry-run" => {
            println!("Dry run (pending migrations):");
            let mut has_pending = false;
            for migration in &migrations {
                let applied: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM _migrations WHERE version = $1)",
                )
                .bind(migration.version() as i64)
                .fetch_one(&pool)
                .await?;

                if !applied {
                    println!("  - {}", migration.name());
                    has_pending = true;
                }
            }
            if !has_pending {
                println!("  No pending migrations.");
            }
        }
        _ => {
            eprintln!("Usage: adi-balance-migrate [all|status|dry-run]");
            std::process::exit(1);
        }
    }

    Ok(())
}

fn redact_password(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let scheme_end = url.find("://").map(|p| p + 3).unwrap_or(0);
            if colon_pos > scheme_end {
                return format!("{}***{}", &url[..colon_pos + 1], &url[at_pos..]);
            }
        }
    }
    url.to_string()
}
