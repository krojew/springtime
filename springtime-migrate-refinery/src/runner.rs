//! Module related to running migrations.

use crate::migration::MigrationSource;
use crate::refinery::Runner;
use itertools::Itertools;
use springtime::future::{BoxFuture, FutureExt};
use springtime::runner::ApplicationRunner;
use springtime_di::instance_provider::{ComponentInstancePtr, ErrorPtr};
use springtime_di::{injectable, Component};
use tracing::{debug, info};

/// Since [Runner] requires a concrete DB client to execute migrations, an abstraction over all
/// possible clients needs to exist, which will execute the actual run operation with a concrete
/// client. This trait is such abstraction. By default, all MigrationRunnerExecutors will be called
/// to run migrations in unspecified order.
#[injectable]
pub trait MigrationRunnerExecutor {
    /// Runs migrations contained in the given [Runner] by passing a concrete DB client.
    fn run_migrations(&self, runner: &Runner) -> BoxFuture<'_, Result<(), ErrorPtr>>;
}

#[derive(Component)]
struct MigrationRunner {
    migration_sources: Vec<ComponentInstancePtr<dyn MigrationSource + Send + Sync>>,
    executors: Vec<ComponentInstancePtr<dyn MigrationRunnerExecutor + Send + Sync>>,
}

#[component_alias]
impl ApplicationRunner for MigrationRunner {
    fn run(&self) -> BoxFuture<'_, Result<(), ErrorPtr>> {
        async {
            if self.migration_sources.is_empty() {
                info!("Not running any migrations, since no sources are available.");
                return Ok(());
            }

            let migrations: Vec<_> = self
                .migration_sources
                .iter()
                .map(|source| source.migrations())
                .flatten_ok()
                .try_collect()?;

            info!("Running {} migrations...", migrations.len());

            let runner = Runner::new(&migrations);
            for executor in &self.executors {
                executor.run_migrations(&runner).await?;
            }

            debug!("Done running migrations.");

            Ok(())
        }
        .boxed()
    }

    fn priority(&self) -> i8 {
        100
    }
}
