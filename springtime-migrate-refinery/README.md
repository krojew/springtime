# Springtime Migrate Refinery

[![crates.io version](https://img.shields.io/crates/v/springtime-migrate-refinery.svg)](https://crates.io/crates/springtime-migrate-refinery)
![build status](https://github.com/krojew/springtime/actions/workflows/rust.yml/badge.svg)

`refinery` is powerful SQL migration toolkit for Rust, which makes creating
migrations easy. This crate integrates `refinery` with the broader [*Springtime
Framework*](https://crates.io/crates/springtime) allowing for providing database
clients and migrations via dependency injection, which further eases creating 
and applying migrations, either from files or Rust code.

Note: in addition to this crate, you need to also import
[springtime-di](https://crates.io/crates/springtime-di).

## Features

* Automatic migration discovery
* File-based and code-based migrations
* Automatic migration application on startup for configured db clients
* All `refinery` db clients supported

## Basic usage

As with `refinery`, the basic usage consists of creating or embedding migrations
and providing a runner for desired database.

The following example assumes familiarity with
[springtime](https://crates.io/crates/springtime) and
[springtime-di](https://crates.io/crates/springtime-di).

```rust
use refinery_core::Runner;
use springtime::application;
use springtime::future::{BoxFuture, FutureExt};
use springtime_di::instance_provider::ErrorPtr;
use springtime_migrate_refinery::migration::embed_migrations;
use springtime_migrate_refinery::runner::MigrationRunnerExecutor;

// this is all that's needed to embed sql migrations from the given folder (the default path is
// "migrations")
embed_migrations!("examples/migrations");

// this is a migration source, which can provide migrations from code, instead of sql files
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

// refinery migration runner needs a concrete DB client to run - this necessitates an abstraction
// layer; please see MigrationRunnerExecutor for details
struct ExampleMigrationRunnerExecutor;

impl MigrationRunnerExecutor for ExampleMigrationRunnerExecutor {
    fn run_migrations(&self, _runner: &Runner) -> BoxFuture<'_, Result<(), ErrorPtr>> {
        // run migrations here with the given runner
        async { Ok(()) }.boxed()
    }
}

// note: for the sake of simplicity, errors are unwrapped, rather than gracefully handled
#[tokio::main]
async fn main() {
    // create our application, which will run refinery migration runner before other runners
    let mut application =
        application::create_default().expect("unable to create default application");

    // will run migrations from the "migrations" folder if MigrationRunnerExecutor(s) are available
    application.run().await.expect("error running application");
}
```
