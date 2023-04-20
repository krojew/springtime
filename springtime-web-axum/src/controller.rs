//! Functionality related to defining [Controller]s - containers for functions which handle web
//! requests.

use axum::Router;
use downcast::{downcast_sync, AnySync};
use fxhash::FxHashSet;
#[cfg(test)]
use mockall::automock;
use springtime_di::injectable;
use springtime_di::instance_provider::ComponentInstancePtr;
use thiserror::Error;

pub type ServerNameSet = FxHashSet<String>;

/// Helper error enum for router configuration errors.
#[derive(Clone, PartialEq, Error, Debug)]
pub enum RouterError {
    #[error("Generic error configuring router: {0}")]
    RouterConfigurationError(String),
}

/// Main trait for [Components](springtime_di::component::Component) used as controllers -
/// collections of web [handlers](axum::handler::Handler) being functions contained in typical
/// structs. Such approach allows for injecting other components via dependency injection, and
/// therefore, creating advanced applications with proper architecture.
#[injectable]
#[cfg_attr(test, automock)]
pub trait Controller: AnySync {
    /// Prefix for all paths contained in the controller, e.g. controller path of `/abc` and handler
    /// path of `/xyz` results in final path of `/abc/xyz`.
    fn path(&self) -> Option<String> {
        None
    }

    /// Optional list of server names for which given controller should be registered.
    fn server_names(&self) -> Option<ServerNameSet> {
        None
    }

    /// Creates a [Router] to handle incoming requests. Passed instance ptr points to the controller
    /// component being processed (`Self`).
    fn configure_router(
        &self,
        self_instance_ptr: ComponentInstancePtr<dyn Controller + Send + Sync>,
    ) -> Result<Router, RouterError>;
}

downcast_sync!(dyn Controller + Send + Sync);
