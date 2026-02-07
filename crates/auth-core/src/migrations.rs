use lib_migrations::SqlMigration;

pub fn migrations() -> Vec<SqlMigration> {
    vec![migration_v1()]
}

fn migration_v1() -> SqlMigration {
    SqlMigration::new(
        1,
        "initial_schema",
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL,
            last_login_at TEXT
        );

        CREATE TABLE IF NOT EXISTS verification_codes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL,
            code TEXT NOT NULL,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
        CREATE INDEX IF NOT EXISTS idx_codes_email ON verification_codes(email);
        CREATE INDEX IF NOT EXISTS idx_codes_expires ON verification_codes(expires_at);
        "#,
    )
    .with_down(
        r#"
        DROP INDEX IF EXISTS idx_codes_expires;
        DROP INDEX IF EXISTS idx_codes_email;
        DROP INDEX IF EXISTS idx_users_email;
        DROP TABLE IF EXISTS verification_codes;
        DROP TABLE IF EXISTS users;
        "#,
    )
}
