pub mod component;
mod error;

pub use error::Error;

#[cfg(feature = "derive")]
pub use springtime_di_derive::Component;
