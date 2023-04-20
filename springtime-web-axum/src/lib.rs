//! Web framework based on [Springtime](https://crates.io/crates/springtime) and axum.
//!
//! `axum` is a web application framework built with a imperative approach - integration with
//! *Springtime* allows for declarative approach to creating handlers (called here
//! [*Controllers*](controller::Controller)) which can take full advantage of dependency injection.
//!
//! ### Features
//!
//! * `derive` - automatically import helper proc macros

pub mod config;
pub mod controller;
pub mod router;
pub mod server;

pub use axum;

#[cfg(feature = "derive")]
pub use springtime_web_axum_derive::*;
