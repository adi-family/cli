lib-migrations-core, rust, migrations, generic, storage-agnostic

## Overview
- Generic migration framework without database dependencies
- User provides storage backend and migration actions
- Supports any context type (database, file system, API, etc.)

## Core Traits
- `Migration<Ctx>` - defines apply/rollback actions for a context type
- `MigrationStore` - tracks which migrations have been applied

## Usage
```rust
use lib_migrations_core::{FnMigration, MigrationRunner, MemoryStore};

let store = MemoryStore::new();
let mut runner = MigrationRunner::new(store)
    .add(FnMigration::new(1, "create_table", |db: &mut Database| {
        db.execute("CREATE TABLE users (...)")
    }))
    .add(FnMigration::new(2, "add_column", |db: &mut Database| {
        db.execute("ALTER TABLE users ADD email TEXT")
    }));

runner.init()?;
runner.migrate(&mut db)?;
```

## Implementing Custom Store
```rust
impl MigrationStore for MyStore {
    fn init(&mut self) -> Result<()> { /* create tracking table */ }
    fn applied(&self) -> Result<Vec<MigrationRecord>> { /* list applied */ }
    fn mark_applied(&mut self, version: u64, name: &str) -> Result<()> { /* record */ }
    fn mark_rolled_back(&mut self, version: u64) -> Result<()> { /* remove */ }
}
```

## Features
- Sequential version validation (1, 2, 3, ...)
- Migrate up/down to specific version
- Status reporting (pending/applied)
- MemoryStore included for testing
