mod error;
mod phase;
mod runner;
mod store;

pub use error::{Error, Result};
pub use phase::Phase;
pub use runner::{DryRunPlan, DryRunResult, MigrationRunner, MigrationStatus};
pub use store::{MemoryStore, MigrationRecord, MigrationStore};

/// A migration that can be applied or rolled back.
///
/// Migrations are versioned changes to your system. Each migration has:
/// - A unique version number (must be sequential starting from 1)
/// - A human-readable name
/// - A deployment phase (pre-deploy or post-deploy)
/// - An apply action
/// - An optional rollback action
///
/// The generic type `Ctx` is the context passed to apply/rollback functions.
/// This could be a database connection, file system handle, API client, etc.
pub trait Migration<Ctx>: Send + Sync {
    /// Unique version number (must be sequential: 1, 2, 3, ...)
    fn version(&self) -> u64;

    /// Human-readable name for this migration
    fn name(&self) -> &str;

    /// Deployment phase (pre-deploy or post-deploy)
    fn phase(&self) -> Phase {
        Phase::PreDeploy
    }

    /// Apply the migration
    fn apply(&self, ctx: &mut Ctx) -> Result<()>;

    /// Rollback the migration (optional)
    fn rollback(&self, ctx: &mut Ctx) -> Result<()> {
        let _ = ctx;
        Err(Error::RollbackNotSupported(self.version()))
    }

    /// Whether this migration supports rollback
    fn can_rollback(&self) -> bool {
        false
    }
}

use std::marker::PhantomData;

/// Builder for creating simple migrations with closures
pub struct FnMigration<Ctx, F, R>
where
    F: Fn(&mut Ctx) -> Result<()> + Send + Sync,
    R: Fn(&mut Ctx) -> Result<()> + Send + Sync,
{
    version: u64,
    name: String,
    phase: Phase,
    apply_fn: F,
    rollback_fn: Option<R>,
    _phantom: PhantomData<fn(&mut Ctx)>, // Use fn pointer for Send+Sync invariance
}

impl<Ctx, F> FnMigration<Ctx, F, fn(&mut Ctx) -> Result<()>>
where
    F: Fn(&mut Ctx) -> Result<()> + Send + Sync,
{
    pub fn new(version: u64, name: impl Into<String>, apply_fn: F) -> Self {
        Self {
            version,
            name: name.into(),
            phase: Phase::PreDeploy,
            apply_fn,
            rollback_fn: None,
            _phantom: PhantomData,
        }
    }
}

impl<Ctx, F, R> FnMigration<Ctx, F, R>
where
    F: Fn(&mut Ctx) -> Result<()> + Send + Sync,
    R: Fn(&mut Ctx) -> Result<()> + Send + Sync,
{
    /// Set the deployment phase
    pub fn phase(mut self, phase: Phase) -> Self {
        self.phase = phase;
        self
    }

    pub fn with_rollback<R2>(self, rollback_fn: R2) -> FnMigration<Ctx, F, R2>
    where
        R2: Fn(&mut Ctx) -> Result<()> + Send + Sync,
    {
        FnMigration {
            version: self.version,
            name: self.name,
            phase: self.phase,
            apply_fn: self.apply_fn,
            rollback_fn: Some(rollback_fn),
            _phantom: PhantomData,
        }
    }
}

impl<Ctx, F, R> Migration<Ctx> for FnMigration<Ctx, F, R>
where
    F: Fn(&mut Ctx) -> Result<()> + Send + Sync,
    R: Fn(&mut Ctx) -> Result<()> + Send + Sync,
{
    fn version(&self) -> u64 {
        self.version
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn phase(&self) -> Phase {
        self.phase
    }

    fn apply(&self, ctx: &mut Ctx) -> Result<()> {
        (self.apply_fn)(ctx)
    }

    fn rollback(&self, ctx: &mut Ctx) -> Result<()> {
        match &self.rollback_fn {
            Some(f) => f(ctx),
            None => Err(Error::RollbackNotSupported(self.version)),
        }
    }

    fn can_rollback(&self) -> bool {
        self.rollback_fn.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fn_migration() {
        let migration = FnMigration::new(1, "test", |ctx: &mut Vec<i32>| {
            ctx.push(1);
            Ok(())
        });

        assert_eq!(migration.version(), 1);
        assert_eq!(migration.name(), "test");
        assert!(!migration.can_rollback());

        let mut ctx = vec![];
        migration.apply(&mut ctx).unwrap();
        assert_eq!(ctx, vec![1]);
    }

    #[test]
    fn test_fn_migration_with_rollback() {
        let migration = FnMigration::new(1, "test", |ctx: &mut Vec<i32>| {
            ctx.push(1);
            Ok(())
        })
        .with_rollback(|ctx: &mut Vec<i32>| {
            ctx.pop();
            Ok(())
        });

        assert!(migration.can_rollback());

        let mut ctx = vec![];
        migration.apply(&mut ctx).unwrap();
        assert_eq!(ctx, vec![1]);

        migration.rollback(&mut ctx).unwrap();
        assert!(ctx.is_empty());
    }
}
