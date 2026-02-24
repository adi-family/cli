//! Analytics migration binary using lib-migrations framework.

use anyhow::Result;
use lib_migrations_core::{MigrationRecord, MigrationRunner, MigrationStore, Phase};
use lib_migrations_sql::{SqlExecutor, SqlMigration};
use sqlx::postgres::{PgPool, PgPoolOptions};

use lib_env_parse::{env_vars, env_opt};

env_vars! {
    DatabaseUrl => "DATABASE_URL",
    PlatformDatabaseUrl => "PLATFORM_DATABASE_URL",
}

// ============================================================================
// PostgreSQL Implementations
// ============================================================================

/// SqlExecutor implementation for sqlx::PgPool
struct PgExecutor<'a> {
    pool: &'a PgPool,
    rt: &'a tokio::runtime::Runtime,
}

impl<'a> SqlExecutor for PgExecutor<'a> {
    type Error = sqlx::Error;

    fn execute(&mut self, sql: &str) -> std::result::Result<(), Self::Error> {
        self.rt
            .block_on(async { sqlx::raw_sql(sql).execute(self.pool).await.map(|_| ()) })
    }
}

/// MigrationStore implementation for PostgreSQL
struct PgStore<'a> {
    pool: &'a PgPool,
    rt: &'a tokio::runtime::Runtime,
}

impl<'a> PgStore<'a> {
    fn new(pool: &'a PgPool, rt: &'a tokio::runtime::Runtime) -> Self {
        Self { pool, rt }
    }
}

impl<'a> MigrationStore for PgStore<'a> {
    fn init(&mut self) -> lib_migrations_core::Result<()> {
        self.rt.block_on(async {
            sqlx::raw_sql(
                r#"
                CREATE TABLE IF NOT EXISTS _migrations (
                    version BIGINT PRIMARY KEY,
                    name VARCHAR(255) NOT NULL,
                    applied_at BIGINT NOT NULL
                )
                "#,
            )
            .execute(self.pool)
            .await
            .map_err(|e| {
                lib_migrations_core::Error::store(format!("Failed to init store: {}", e))
            })?;
            Ok(())
        })
    }

    fn applied(&self) -> lib_migrations_core::Result<Vec<MigrationRecord>> {
        self.rt.block_on(async {
            let records: Vec<(i64, String, i64)> = sqlx::query_as(
                "SELECT version, name, applied_at FROM _migrations ORDER BY version",
            )
            .fetch_all(self.pool)
            .await
            .map_err(|e| {
                lib_migrations_core::Error::store(format!(
                    "Failed to fetch applied migrations: {}",
                    e
                ))
            })?;

            Ok(records
                .into_iter()
                .map(|(version, name, applied_at)| MigrationRecord {
                    version: version as u64,
                    name,
                    applied_at: applied_at as u64,
                })
                .collect())
        })
    }

    fn mark_applied(&mut self, version: u64, name: &str) -> lib_migrations_core::Result<()> {
        self.rt.block_on(async {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            sqlx::query("INSERT INTO _migrations (version, name, applied_at) VALUES ($1, $2, $3)")
                .bind(version as i64)
                .bind(name)
                .bind(now as i64)
                .execute(self.pool)
                .await
                .map_err(|e| {
                    lib_migrations_core::Error::store(format!(
                        "Failed to mark migration {} as applied: {}",
                        version, e
                    ))
                })?;
            Ok(())
        })
    }

    fn mark_rolled_back(&mut self, version: u64) -> lib_migrations_core::Result<()> {
        self.rt.block_on(async {
            sqlx::query("DELETE FROM _migrations WHERE version = $1")
                .bind(version as i64)
                .execute(self.pool)
                .await
                .map_err(|e| {
                    lib_migrations_core::Error::store(format!(
                        "Failed to mark migration {} as rolled back: {}",
                        version, e
                    ))
                })?;
            Ok(())
        })
    }
}

// ============================================================================
// Migrations
// ============================================================================

const MIGRATION_001: &str = include_str!("../../migrations/001_create_analytics_events.sql");
const MIGRATION_002: &str = include_str!("../../migrations/002_create_analytics_aggregates.sql");

fn register_migrations() -> Vec<SqlMigration> {
    vec![
        SqlMigration::new(1, "001_create_analytics_events", MIGRATION_001)
            .phase(Phase::PreDeploy),
        SqlMigration::new(2, "002_create_analytics_aggregates", MIGRATION_002)
            .phase(Phase::PostDeploy),
    ]
}

// ============================================================================
// Main
// ============================================================================

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = env_opt(EnvVar::DatabaseUrl.as_str())
        .or_else(|| env_opt(EnvVar::PlatformDatabaseUrl.as_str()))
        .expect("DATABASE_URL or PLATFORM_DATABASE_URL must be set");

    // Create a runtime for async database operations
    let rt = tokio::runtime::Runtime::new()?;
    let pool = rt.block_on(async {
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
    })?;

    let store = PgStore::new(&pool, &rt);
    let mut runner = MigrationRunner::new(store);

    // Register all migrations
    for migration in register_migrations() {
        runner = runner.add(migration);
    }

    // Initialize migration tracking
    runner.init()?;

    // Parse command
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("all");

    match command {
        "pre" => {
            println!("Running pre-deploy migrations...");
            let mut executor = PgExecutor {
                pool: &pool,
                rt: &rt,
            };
            let count = runner.migrate_phase(&mut executor, Phase::PreDeploy)?;
            println!("✓ Applied {} pre-deploy migration(s)", count);
        }
        "post" => {
            println!("Running post-deploy migrations...");
            let mut executor = PgExecutor {
                pool: &pool,
                rt: &rt,
            };
            let count = runner.migrate_phase(&mut executor, Phase::PostDeploy)?;
            println!("✓ Applied {} post-deploy migration(s)", count);
        }
        "all" => {
            println!("Running all migrations...");
            let mut executor = PgExecutor {
                pool: &pool,
                rt: &rt,
            };
            let count = runner.migrate(&mut executor)?;
            println!("✓ Applied {} migration(s)", count);
        }
        "status" => {
            let status = runner.status()?;
            println!("Migration Status:");
            println!("  Total: {}", status.len());
            let applied_count = status.iter().filter(|s| s.applied).count();
            let pending_count = status.len() - applied_count;
            println!("  Applied: {}", applied_count);
            println!("  Pending: {}", pending_count);

            if pending_count > 0 {
                println!("\nPending migrations:");
                for mig in status.iter().filter(|s| !s.applied) {
                    println!("  - {} (version {}, {:?})", mig.name, mig.version, mig.phase);
                }
            }
        }
        "dry-run" => {
            let plan = runner.dry_run()?;
            if plan.is_empty() {
                println!("No pending migrations.");
            } else {
                println!("Would apply {} migration(s):", plan.total);
                for mig in plan.pending {
                    println!(
                        "  - {} (version {}, {:?})",
                        mig.name, mig.version, mig.phase
                    );
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Usage: analytics-migrate <pre|post|all|status|dry-run>");
            std::process::exit(1);
        }
    }

    Ok(())
}
