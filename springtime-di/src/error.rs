use thiserror::Error;

/// Errors related to creating and managing components.
#[derive(Error, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum Error {
    #[error("Cannot find a primary instance for component '{0}' - either none or multiple exists without a primary marker.")]
    NoPrimaryInstance(String),
}
