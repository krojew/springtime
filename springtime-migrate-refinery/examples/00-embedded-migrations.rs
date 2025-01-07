use refinery_core::Runner;
use springtime::application;
use springtime::future::{BoxFuture, FutureExt};
use springtime_di::instance_provider::ErrorPtr;
use springtime_migrate_refinery::migration::embed_migrations;
use springtime_migrate_refinery::runner::MigrationRunnerExecutor;

// this is all that's needed to embed SQL migrations from the given folder (the default path is
// "migrations")
// when building this example, the current working directory is the workspace one
embed_migrations!("./springtime-migrate-refinery/examples/migrations");

// refinery migration runner needs a concrete DB client to run - this requires an abstraction
// layer; please see MigrationRunnerExecutor for details
#[allow(dead_code)]
struct ExampleMigrationRunnerExecutor;

impl MigrationRunnerExecutor for ExampleMigrationRunnerExecutor {
    fn run_migrations(&self, _runner: &Runner) -> BoxFuture<'_, Result<(), ErrorPtr>> {
        // run migrations here with the given runner
        async { Ok(()) }.boxed()
    }
}

// note: for the sake of simplicity, errors are unwrapped rather than gracefully handled
#[tokio::main]
async fn main() {
    // create our application, which will run the refinery migration runner before other runners
    let mut application =
        application::create_default().expect("unable to create default application");

    // will run migrations from the "migrations" folder if MigrationRunnerExecutor(s) are available
    application.run().await.expect("error running application");
}
