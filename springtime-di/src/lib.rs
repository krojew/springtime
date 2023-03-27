//! A dependency injection crate inspired by the [Spring Framework](https://spring.io/) in Java.
//!
//! The philosophy of *Springtime* is to provide the means of easy dependency injection without
//! unnecessary manual configuration, e.g. without the need to explicitly create dependencies and
//! storing them in containers. As much work as possible is placed on compile-time metadata creation
//! and automatic [component] discovery, thus allowing users to focus on the _usage_ of components,
//! rather than their _creation_ and _management_. With an accent placed on attributes,
//! dependency configuration becomes declarative (_what I want to accomplish_) leaving the gritty
//! details the the framework itself (_how to accomplish what was requested_).
//!
//! ### Simple usage example
//!
//! ```
//! use springtime_di::component::Component;
//! use springtime_di::instance_provider::ComponentInstancePtr;
//! use springtime_di::{Component, component_alias};
//!
//! // this is a trait we would like to use in our component
//! trait TestTrait {}
//!
//! // this is a dependency which implements the above trait and also an injectable component
//! #[derive(Component)]
//! struct TestDependency;
//!
//! // we're telling the framework it should provide TestDependency when asked for dyn TestTrait
//! #[component_alias]
//! impl TestTrait for TestDependency {}
//!
//! // this is another component, but with a dependency
//! #[derive(Component)]
//! struct TestComponent {
//!     // the framework will know how to inject dyn TestTrait, when asked for TestComponent
//!     // more details are available in other parts of the documentation
//!     dependency: ComponentInstancePtr<dyn TestTrait + Send + Sync>,
//! }
//! ```
//!
//! *Note:* `Send + Sync` are only required when the `threadsafe` feature is enabled.

pub mod component;
pub mod component_registry;
pub mod error;
pub mod instance_provider;

#[cfg(feature = "derive")]
pub use springtime_di_derive::*;
