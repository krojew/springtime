pub mod component;
pub mod component_registry;
pub mod error;

#[cfg(feature = "derive")]
pub use springtime_di_derive::Component;
