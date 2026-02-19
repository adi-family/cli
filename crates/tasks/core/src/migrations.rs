use lib_migrations::SqlMigration;

pub fn migrations() -> Vec<SqlMigration> {
    vec![migration_v1()]
}

fn migration_v1() -> SqlMigration {
    SqlMigration::new(
        1,
        "initial_schema",
        r#"
        -- Main tasks table
        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL DEFAULT 'todo',
            symbol_id INTEGER,
            project_path TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        -- Task dependencies (from depends on to)
        CREATE TABLE IF NOT EXISTS task_dependencies (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            from_task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            to_task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            UNIQUE(from_task_id, to_task_id)
        );

        -- Indexes
        CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
        CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_path);
        CREATE INDEX IF NOT EXISTS idx_tasks_symbol ON tasks(symbol_id);
        CREATE INDEX IF NOT EXISTS idx_deps_from ON task_dependencies(from_task_id);
        CREATE INDEX IF NOT EXISTS idx_deps_to ON task_dependencies(to_task_id);

        -- FTS5 virtual table for full-text search
        CREATE VIRTUAL TABLE IF NOT EXISTS tasks_fts USING fts5(
            title,
            description,
            content='tasks',
            content_rowid='id'
        );

        -- FTS sync triggers
        CREATE TRIGGER IF NOT EXISTS tasks_ai AFTER INSERT ON tasks BEGIN
            INSERT INTO tasks_fts(rowid, title, description)
            VALUES (new.id, new.title, new.description);
        END;

        CREATE TRIGGER IF NOT EXISTS tasks_ad AFTER DELETE ON tasks BEGIN
            INSERT INTO tasks_fts(tasks_fts, rowid, title, description)
            VALUES ('delete', old.id, old.title, old.description);
        END;

        CREATE TRIGGER IF NOT EXISTS tasks_au AFTER UPDATE ON tasks BEGIN
            INSERT INTO tasks_fts(tasks_fts, rowid, title, description)
            VALUES ('delete', old.id, old.title, old.description);
            INSERT INTO tasks_fts(rowid, title, description)
            VALUES (new.id, new.title, new.description);
        END;
        "#,
    )
    .with_down(
        r#"
        DROP TRIGGER IF EXISTS tasks_au;
        DROP TRIGGER IF EXISTS tasks_ad;
        DROP TRIGGER IF EXISTS tasks_ai;
        DROP TABLE IF EXISTS tasks_fts;
        DROP INDEX IF EXISTS idx_deps_to;
        DROP INDEX IF EXISTS idx_deps_from;
        DROP INDEX IF EXISTS idx_tasks_symbol;
        DROP INDEX IF EXISTS idx_tasks_project;
        DROP INDEX IF EXISTS idx_tasks_status;
        DROP TABLE IF EXISTS task_dependencies;
        DROP TABLE IF EXISTS tasks;
        "#,
    )
}
