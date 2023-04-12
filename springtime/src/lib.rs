//! Application framework based on [springtime_di] dependency injection.
//!
//! Traditional applications start in the `main()` function and often explicitly initialize and pass
//! around various domain/application services or other components. With dependency injection in
//! place, all application components can become decoupled and form a dependency graph managed by a
//! DI framework. This, in turn, requires an entrypoint for the application which initializes DI and
//! runs the actual business logic of the application. This crate provides such entrypoint in the
//! form of [Application](application::Application), which also configures additional supporting
//! infrastructure, e.g. logging.
//!
//! ### Simple usage example
//!
//! ```
//! use springtime::application;
//! use springtime::runner::{ApplicationRunner, BoxFuture};
//! use springtime_di::future::FutureExt;
//! use springtime_di::instance_provider::ErrorPtr;
//! use springtime_di::{component_alias, Component};
//!
//! // this is an application runner, which will run when the application starts; the framework will
//! // automatically discover it using dependency injection
//! #[derive(Component)]
//! struct HelloWorldRunner;
//!
//! //noinspection DuplicatedCode
//! #[component_alias]
//! impl ApplicationRunner for HelloWorldRunner {
//!     // note: BoxFuture is only needed when using the "async" feature
//!     fn run(&self) -> BoxFuture<'_, Result<(), ErrorPtr>> {
//!         async {
//!             println!("Hello world!");
//!             Ok(())
//!         }
//!         .boxed()
//!     }
//! }
//!
//! // note: for the sake of simplicity, errors are unwrapped, rather than gracefully handled
//! #[tokio::main]
//! async fn main() {
//!     // create our application, which will detect all runners
//!     let mut application =
//!         application::create_default().expect("unable to create default application");
//!
//!     // prints "Hello world!"
//!     application.run().await.expect("error running application");
//! }
//! ```
//!
//! ### Features
//!
//! * `threadsafe` - use threadsafe pointers and `Send + Sync` trait bounds
//! * `async` - turn all run functions async

pub mod application;
pub mod config;
pub mod runner;
