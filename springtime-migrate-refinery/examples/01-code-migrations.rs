// note: this example assumes you've analyzed the previous one

use springtime::application;
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, Component};
use springtime_migrate_refinery::migration::MigrationSource;
use springtime_migrate_refinery::refinery::Migration;
use std::sync::Arc;

// this is a migration source, which can provide migrations from code
#[derive(Component)]
struct ExampleMigrationSource;

// register the source with dependency injection
#[component_alias]
impl MigrationSource for ExampleMigrationSource {
    fn migrations(&self) -> Result<Vec<Migration>, ErrorPtr> {
        Migration::unapplied("V00__test", "CREATE TABLE test (id INTEGER PRIMARY KEY);")
            .map(|migration| vec![migration])
            .map_err(|error| Arc::new(error) as ErrorPtr)
    }
}

// note: for the sake of simplicity, errors are unwrapped, rather than gracefully handled
#[tokio::main]
async fn main() {
    let mut application =
        application::create_default().expect("unable to create default application");

    application.run().await.expect("error running application");
}
