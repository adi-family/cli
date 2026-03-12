use crate::error::{Error, Result};
use crate::phase::Phase;
use crate::store::MigrationStore;
use crate::Migration;

/// Status of a migration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationStatus {
    pub version: u64,
    pub name: String,
    pub phase: Phase,
    pub applied: bool,
    pub applied_at: Option<u64>,
}

/// A planned migration for dry-run output
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DryRunPlan {
    pub version: u64,
    pub name: String,
    pub phase: Phase,
    pub can_rollback: bool,
}

/// Result of a dry-run
#[derive(Debug, Clone)]
pub struct DryRunResult {
    pub pending: Vec<DryRunPlan>,
    pub total: usize,
}

impl DryRunResult {
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// Runs migrations against a context using a store for tracking.
///
/// Generic parameters:
/// - `Ctx`: The context passed to migrations (e.g., database connection)
/// - `S`: The store implementation for tracking applied migrations
pub struct MigrationRunner<'a, Ctx, S: MigrationStore> {
    store: S,
    migrations: Vec<Box<dyn Migration<Ctx> + 'a>>,
}

impl<'a, Ctx, S: MigrationStore> MigrationRunner<'a, Ctx, S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            migrations: Vec::new(),
        }
    }

    /// Consume the runner and return the store
    pub fn into_store(self) -> S {
        self.store
    }

    /// Get a reference to the store
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Get a mutable reference to the store
    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    /// Add a migration
    pub fn add<M: Migration<Ctx> + 'a>(mut self, migration: M) -> Self {
        self.migrations.push(Box::new(migration));
        self
    }

    /// Add multiple migrations
    pub fn add_all<I, M>(mut self, migrations: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Migration<Ctx> + 'a,
    {
        for m in migrations {
            self.migrations.push(Box::new(m));
        }
        self
    }

    /// Initialize store and validate migrations
    pub fn init(&mut self) -> Result<()> {
        self.store.init()?;
        self.validate()?;
        Ok(())
    }

    /// Validate migration versions are sequential (1, 2, 3, ...)
    fn validate(&self) -> Result<()> {
        let mut versions: Vec<u64> = self.migrations.iter().map(|m| m.version()).collect();
        versions.sort();
        versions.dedup();

        if versions.len() != self.migrations.len() {
            return Err(Error::InvalidOrder("Duplicate migration versions".into()));
        }

        for (i, &version) in versions.iter().enumerate() {
            let expected = (i + 1) as u64;
            if version != expected {
                return Err(Error::InvalidOrder(format!(
                    "Expected version {}, found {}. Versions must be sequential.",
                    expected, version
                )));
            }
        }

        Ok(())
    }

    /// Get current schema version
    pub fn current_version(&self) -> Result<u64> {
        self.store.current_version()
    }

    /// Get status of all migrations
    pub fn status(&self) -> Result<Vec<MigrationStatus>> {
        let applied = self.store.applied()?;
        let applied_map: std::collections::HashMap<u64, u64> =
            applied.into_iter().map(|r| (r.version, r.applied_at)).collect();

        let mut statuses: Vec<MigrationStatus> = self
            .migrations
            .iter()
            .map(|m| {
                let version = m.version();
                let applied_at = applied_map.get(&version).copied();
                MigrationStatus {
                    version,
                    name: m.name().to_string(),
                    phase: m.phase(),
                    applied: applied_at.is_some(),
                    applied_at,
                }
            })
            .collect();

        statuses.sort_by_key(|s| s.version);
        Ok(statuses)
    }

    /// Get pending (unapplied) migrations
    pub fn pending(&self) -> Result<Vec<&dyn Migration<Ctx>>> {
        let current = self.store.current_version()?;
        let mut pending: Vec<&dyn Migration<Ctx>> = self
            .migrations
            .iter()
            .filter(|m| m.version() > current)
            .map(|m| m.as_ref())
            .collect();

        pending.sort_by_key(|m| m.version());
        Ok(pending)
    }

    /// Get pending migrations for a specific phase
    pub fn pending_phase(&self, phase: Phase) -> Result<Vec<&dyn Migration<Ctx>>> {
        let current = self.store.current_version()?;
        let mut pending: Vec<&dyn Migration<Ctx>> = self
            .migrations
            .iter()
            .filter(|m| m.version() > current && m.phase() == phase)
            .map(|m| m.as_ref())
            .collect();

        pending.sort_by_key(|m| m.version());
        Ok(pending)
    }

    /// Run all pending migrations
    pub fn migrate(&mut self, ctx: &mut Ctx) -> Result<usize> {
        let pending: Vec<u64> = self.pending()?.iter().map(|m| m.version()).collect();
        let count = pending.len();

        for version in pending {
            self.apply_version(ctx, version)?;
        }

        Ok(count)
    }

    /// Run pending migrations for a specific phase only
    pub fn migrate_phase(&mut self, ctx: &mut Ctx, phase: Phase) -> Result<usize> {
        let pending: Vec<u64> = self
            .pending_phase(phase)?
            .iter()
            .map(|m| m.version())
            .collect();
        let count = pending.len();

        for version in pending {
            self.apply_version(ctx, version)?;
        }

        Ok(count)
    }

    /// Dry-run: show what would be applied without actually applying
    pub fn dry_run(&self) -> Result<DryRunResult> {
        let pending = self.pending()?;

        let plans: Vec<DryRunPlan> = pending
            .iter()
            .map(|m| DryRunPlan {
                version: m.version(),
                name: m.name().to_string(),
                phase: m.phase(),
                can_rollback: m.can_rollback(),
            })
            .collect();

        Ok(DryRunResult {
            total: plans.len(),
            pending: plans,
        })
    }

    /// Dry-run for a specific phase
    pub fn dry_run_phase(&self, phase: Phase) -> Result<DryRunResult> {
        let pending = self.pending_phase(phase)?;

        let plans: Vec<DryRunPlan> = pending
            .iter()
            .map(|m| DryRunPlan {
                version: m.version(),
                name: m.name().to_string(),
                phase: m.phase(),
                can_rollback: m.can_rollback(),
            })
            .collect();

        Ok(DryRunResult {
            total: plans.len(),
            pending: plans,
        })
    }

    /// Migrate to a specific version (up or down)
    pub fn migrate_to(&mut self, ctx: &mut Ctx, target: u64) -> Result<usize> {
        let current = self.store.current_version()?;
        let mut count = 0;

        if target > current {
            // Migrate up
            let to_apply: Vec<u64> = self
                .migrations
                .iter()
                .filter(|m| m.version() > current && m.version() <= target)
                .map(|m| m.version())
                .collect();

            for version in to_apply {
                self.apply_version(ctx, version)?;
                count += 1;
            }
        } else if target < current {
            // Migrate down
            let mut to_rollback: Vec<u64> = self
                .migrations
                .iter()
                .filter(|m| m.version() > target && m.version() <= current)
                .map(|m| m.version())
                .collect();

            to_rollback.sort_by(|a, b| b.cmp(a)); // Descending

            for version in to_rollback {
                self.rollback_version(ctx, version)?;
                count += 1;
            }
        }

        Ok(count)
    }

    /// Apply a specific migration by version
    fn apply_version(&mut self, ctx: &mut Ctx, version: u64) -> Result<()> {
        let migration = self
            .migrations
            .iter()
            .find(|m| m.version() == version)
            .ok_or(Error::NotFound(version))?;

        let name = migration.name().to_string();

        migration.apply(ctx).map_err(|e| Error::MigrationFailed {
            version,
            message: e.to_string(),
        })?;

        self.store.mark_applied(version, &name)?;
        Ok(())
    }

    /// Rollback a specific migration by version
    fn rollback_version(&mut self, ctx: &mut Ctx, version: u64) -> Result<()> {
        let migration = self
            .migrations
            .iter()
            .find(|m| m.version() == version)
            .ok_or(Error::NotFound(version))?;

        if !migration.can_rollback() {
            return Err(Error::RollbackNotSupported(version));
        }

        migration.rollback(ctx).map_err(|e| Error::MigrationFailed {
            version,
            message: e.to_string(),
        })?;

        self.store.mark_rolled_back(version)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryStore;
    use crate::FnMigration;

    #[test]
    fn test_runner_migrate() {
        let store = MemoryStore::new();
        let mut runner = MigrationRunner::new(store)
            .add(FnMigration::new(1, "first", |ctx: &mut Vec<i32>| {
                ctx.push(1);
                Ok(())
            }))
            .add(FnMigration::new(2, "second", |ctx: &mut Vec<i32>| {
                ctx.push(2);
                Ok(())
            }));

        runner.init().unwrap();

        let mut ctx = vec![];
        let count = runner.migrate(&mut ctx).unwrap();

        assert_eq!(count, 2);
        assert_eq!(ctx, vec![1, 2]);
        assert_eq!(runner.current_version().unwrap(), 2);
    }

    #[test]
    fn test_runner_migrate_phase() {
        let store = MemoryStore::new();
        let mut runner = MigrationRunner::new(store)
            .add(FnMigration::new(1, "pre_1", |ctx: &mut Vec<&str>| {
                ctx.push("pre_1");
                Ok(())
            }))
            .add(
                FnMigration::new(2, "post_1", |ctx: &mut Vec<&str>| {
                    ctx.push("post_1");
                    Ok(())
                })
                .phase(Phase::PostDeploy),
            )
            .add(FnMigration::new(3, "pre_2", |ctx: &mut Vec<&str>| {
                ctx.push("pre_2");
                Ok(())
            }));

        runner.init().unwrap();

        let mut ctx = vec![];

        // Run only pre-deploy
        let count = runner.migrate_phase(&mut ctx, Phase::PreDeploy).unwrap();
        assert_eq!(count, 2); // pre_1 and pre_2
        assert_eq!(ctx, vec!["pre_1", "pre_2"]);
        assert_eq!(runner.current_version().unwrap(), 3);

        // post_1 was skipped, so pending post-deploy is empty
        // (because version 2 is already past current_version check)
        let pending_post = runner.pending_phase(Phase::PostDeploy).unwrap();
        assert_eq!(pending_post.len(), 0);
    }

    #[test]
    fn test_runner_dry_run() {
        let store = MemoryStore::new();
        let mut runner = MigrationRunner::new(store)
            .add(FnMigration::new(1, "first", |_: &mut ()| Ok(())))
            .add(
                FnMigration::new(2, "second", |_: &mut ()| Ok(()))
                    .phase(Phase::PostDeploy)
                    .with_rollback(|_: &mut ()| Ok(())),
            );

        runner.init().unwrap();

        let result = runner.dry_run().unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.pending[0].version, 1);
        assert_eq!(result.pending[0].phase, Phase::PreDeploy);
        assert!(!result.pending[0].can_rollback);
        assert_eq!(result.pending[1].version, 2);
        assert_eq!(result.pending[1].phase, Phase::PostDeploy);
        assert!(result.pending[1].can_rollback);
    }

    #[test]
    fn test_runner_dry_run_phase() {
        let store = MemoryStore::new();
        let mut runner = MigrationRunner::new(store)
            .add(FnMigration::new(1, "pre_1", |_: &mut ()| Ok(())))
            .add(
                FnMigration::new(2, "post_1", |_: &mut ()| Ok(())).phase(Phase::PostDeploy),
            )
            .add(FnMigration::new(3, "pre_2", |_: &mut ()| Ok(())));

        runner.init().unwrap();

        let pre_result = runner.dry_run_phase(Phase::PreDeploy).unwrap();
        assert_eq!(pre_result.total, 2);
        assert_eq!(pre_result.pending[0].name, "pre_1");
        assert_eq!(pre_result.pending[1].name, "pre_2");

        let post_result = runner.dry_run_phase(Phase::PostDeploy).unwrap();
        assert_eq!(post_result.total, 1);
        assert_eq!(post_result.pending[0].name, "post_1");
    }

    #[test]
    fn test_runner_migrate_to() {
        let store = MemoryStore::new();
        let mut runner = MigrationRunner::new(store)
            .add(
                FnMigration::new(1, "first", |ctx: &mut Vec<i32>| {
                    ctx.push(1);
                    Ok(())
                })
                .with_rollback(|ctx: &mut Vec<i32>| {
                    ctx.retain(|&x| x != 1);
                    Ok(())
                }),
            )
            .add(
                FnMigration::new(2, "second", |ctx: &mut Vec<i32>| {
                    ctx.push(2);
                    Ok(())
                })
                .with_rollback(|ctx: &mut Vec<i32>| {
                    ctx.retain(|&x| x != 2);
                    Ok(())
                }),
            );

        runner.init().unwrap();

        let mut ctx = vec![];

        // Migrate to version 2
        runner.migrate_to(&mut ctx, 2).unwrap();
        assert_eq!(ctx, vec![1, 2]);
        assert_eq!(runner.current_version().unwrap(), 2);

        // Rollback to version 1
        runner.migrate_to(&mut ctx, 1).unwrap();
        assert_eq!(ctx, vec![1]);
        assert_eq!(runner.current_version().unwrap(), 1);

        // Rollback to version 0
        runner.migrate_to(&mut ctx, 0).unwrap();
        assert!(ctx.is_empty());
        assert_eq!(runner.current_version().unwrap(), 0);
    }

    #[test]
    fn test_runner_status() {
        let store = MemoryStore::new();
        let mut runner = MigrationRunner::new(store)
            .add(FnMigration::new(1, "first", |_: &mut ()| Ok(())))
            .add(
                FnMigration::new(2, "second", |_: &mut ()| Ok(())).phase(Phase::PostDeploy),
            );

        runner.init().unwrap();

        let status = runner.status().unwrap();
        assert_eq!(status.len(), 2);
        assert!(!status[0].applied);
        assert_eq!(status[0].phase, Phase::PreDeploy);
        assert!(!status[1].applied);
        assert_eq!(status[1].phase, Phase::PostDeploy);

        runner.migrate(&mut ()).unwrap();

        let status = runner.status().unwrap();
        assert!(status[0].applied);
        assert!(status[1].applied);
    }

    #[test]
    fn test_runner_validates_order() {
        let store = MemoryStore::new();
        let mut runner = MigrationRunner::new(store)
            .add(FnMigration::new(1, "first", |_: &mut ()| Ok(())))
            .add(FnMigration::new(3, "third", |_: &mut ()| Ok(()))); // Skip 2

        let result = runner.init();
        assert!(matches!(result, Err(Error::InvalidOrder(_))));
    }

    #[test]
    fn test_runner_validates_duplicates() {
        let store = MemoryStore::new();
        let mut runner = MigrationRunner::new(store)
            .add(FnMigration::new(1, "first", |_: &mut ()| Ok(())))
            .add(FnMigration::new(1, "duplicate", |_: &mut ()| Ok(()))); // Duplicate

        let result = runner.init();
        assert!(matches!(result, Err(Error::InvalidOrder(_))));
    }
}
