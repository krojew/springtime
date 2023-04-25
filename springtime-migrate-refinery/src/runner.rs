//! Module related to running migrations.

use crate::config::MigrationConfigProvider;
use crate::migration::MigrationSource;
use crate::refinery::Runner;
use itertools::Itertools;
#[cfg(test)]
use mockall::automock;
use springtime::future::{BoxFuture, FutureExt};
use springtime::runner::ApplicationRunner;
use springtime_di::instance_provider::{ComponentInstancePtr, ErrorPtr};
use springtime_di::{component_alias, injectable, Component};
use tracing::{debug, info};

/// Since [Runner] requires a concrete DB client to execute migrations, an abstraction over all
/// possible clients needs to exist, which will execute the actual run operation with a concrete
/// client. This trait is such abstraction. By default, all MigrationRunnerExecutors will be called
/// to run migrations in unspecified order.
#[injectable]
#[cfg_attr(test, automock)]
pub trait MigrationRunnerExecutor {
    /// Runs migrations contained in the given [Runner] by passing a concrete DB client.
    fn run_migrations(&self, runner: &Runner) -> BoxFuture<'_, Result<(), ErrorPtr>>;
}

#[derive(Component)]
struct MigrationRunner {
    config_provider: ComponentInstancePtr<dyn MigrationConfigProvider + Send + Sync>,
    migration_sources: Vec<ComponentInstancePtr<dyn MigrationSource + Send + Sync>>,
    executors: Vec<ComponentInstancePtr<dyn MigrationRunnerExecutor + Send + Sync>>,
}

#[component_alias]
impl ApplicationRunner for MigrationRunner {
    fn run(&self) -> BoxFuture<'_, Result<(), ErrorPtr>> {
        async {
            let config = self.config_provider.config().await?;
            if !config.run_migrations_on_start {
                debug!("Migrations disabled.");
                return Ok(());
            }

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

            let mut runner = Runner::new(&migrations)
                .set_target(config.target.into())
                .set_grouped(config.grouped)
                .set_abort_divergent(config.abort_divergent)
                .set_abort_missing(config.abort_missing);
            runner.set_migration_table_name(&config.migration_table_name);

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

#[cfg(test)]
mod tests {
    use crate::config::{MigrationConfig, MigrationConfigProvider};
    use crate::migration::MockMigrationSource;
    use crate::runner::{MigrationRunner, MockMigrationRunnerExecutor};
    use refinery_core::Migration;
    use springtime::future::{BoxFuture, FutureExt};
    use springtime::runner::ApplicationRunner;
    use springtime_di::instance_provider::{ComponentInstancePtr, ErrorPtr};

    #[derive(Default)]
    struct TestMigrationConfigProvider {
        config: MigrationConfig,
    }

    impl MigrationConfigProvider for TestMigrationConfigProvider {
        fn config(&self) -> BoxFuture<'_, Result<&MigrationConfig, ErrorPtr>> {
            async { Ok(&self.config) }.boxed()
        }
    }

    #[tokio::test]
    async fn should_execute_migrations() {
        let mut migration_source = MockMigrationSource::new();
        migration_source
            .expect_migrations()
            .times(1)
            .return_const(Ok(vec![Migration::unapplied("V00__test", "test").unwrap()]));

        let mut executor = MockMigrationRunnerExecutor::new();
        executor
            .expect_run_migrations()
            .times(1)
            .returning(|_| async { Ok(()) }.boxed());

        let runner = MigrationRunner {
            config_provider: ComponentInstancePtr::new(TestMigrationConfigProvider::default()),
            migration_sources: vec![ComponentInstancePtr::new(migration_source)],
            executors: vec![ComponentInstancePtr::new(executor)],
        };
        runner.run().await.unwrap();
    }
}
