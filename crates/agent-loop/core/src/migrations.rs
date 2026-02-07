use lib_migrations::SqlMigration;

/// Returns all migrations for adi-agent-loop-core
pub fn migrations() -> Vec<SqlMigration> {
    vec![migration_v1()]
}

/// V1: Initial schema with sessions table and FTS
fn migration_v1() -> SqlMigration {
    SqlMigration::new(
        1,
        "initial_sessions_schema",
        r#"
        -- Main sessions table
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            project_path TEXT,
            system_prompt TEXT,
            messages TEXT NOT NULL DEFAULT '[]',
            loop_config TEXT NOT NULL,
            loop_state TEXT NOT NULL,
            error_message TEXT,
            metadata TEXT NOT NULL DEFAULT 'null',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        -- Indexes for common queries
        CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
        CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project_path);
        CREATE INDEX IF NOT EXISTS idx_sessions_created ON sessions(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC);

        -- FTS5 virtual table for full-text search
        CREATE VIRTUAL TABLE IF NOT EXISTS sessions_fts USING fts5(
            title,
            description,
            content='sessions',
            content_rowid='rowid'
        );

        -- FTS sync triggers
        CREATE TRIGGER IF NOT EXISTS sessions_ai AFTER INSERT ON sessions BEGIN
            INSERT INTO sessions_fts(rowid, title, description)
            SELECT rowid, new.title, new.description FROM sessions WHERE id = new.id;
        END;

        CREATE TRIGGER IF NOT EXISTS sessions_ad AFTER DELETE ON sessions BEGIN
            INSERT INTO sessions_fts(sessions_fts, rowid, title, description)
            SELECT 'delete', rowid, old.title, old.description FROM sessions WHERE id = old.id;
        END;

        CREATE TRIGGER IF NOT EXISTS sessions_au AFTER UPDATE ON sessions BEGIN
            INSERT INTO sessions_fts(sessions_fts, rowid, title, description)
            SELECT 'delete', rowid, old.title, old.description FROM sessions WHERE id = old.id;
            INSERT INTO sessions_fts(rowid, title, description)
            SELECT rowid, new.title, new.description FROM sessions WHERE id = new.id;
        END;
        "#,
    )
    .with_down(
        r#"
        DROP TRIGGER IF EXISTS sessions_au;
        DROP TRIGGER IF EXISTS sessions_ad;
        DROP TRIGGER IF EXISTS sessions_ai;
        DROP TABLE IF EXISTS sessions_fts;
        DROP INDEX IF EXISTS idx_sessions_updated;
        DROP INDEX IF EXISTS idx_sessions_created;
        DROP INDEX IF EXISTS idx_sessions_project;
        DROP INDEX IF EXISTS idx_sessions_status;
        DROP TABLE IF EXISTS sessions;
        "#,
    )
}
