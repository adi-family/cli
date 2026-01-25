//! Migration binary for logging database schema.

use lib_migrations_core::cli::run_cli;
use lib_migrations_sql::SqlMigrationSource;

fn main() {
    dotenvy::dotenv().ok();

    let migrations_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");
    let source = SqlMigrationSource::new(migrations_dir);

    if let Err(e) = run_cli(source, "logging") {
        eprintln!("Migration error: {}", e);
        std::process::exit(1);
    }
}
