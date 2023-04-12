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
//! ### Features
//!
//! * `threadsafe` - use threadsafe pointers and `Send + Sync` trait bounds
//! * `async` - turn all run functions async

pub mod application;
pub mod config;
pub mod runner;
