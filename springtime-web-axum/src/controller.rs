//! Functionality related to defining [Controller]s - containers for functions which handle web
//! requests.

use axum::Router;
use downcast::{downcast_sync, AnySync};
use fxhash::FxHashSet;
#[cfg(test)]
use mockall::automock;
use springtime_di::injectable;
use springtime_di::instance_provider::{ComponentInstancePtr, ErrorPtr};

pub type ServerNameSet = FxHashSet<String>;

/// The main trait for [Components](springtime_di::component::Component) used as controllers -
/// collections of web [handlers](axum::handler::Handler) being functions contained in typical
/// structs.
/// This approach allows for injecting other components via dependency injection, and
/// therefore, creating advanced applications with proper architecture.
#[injectable]
#[cfg_attr(test, automock)]
pub trait Controller: AnySync {
    /// Prefix for all paths contained in the controller, e.g., controller path of `/abc` and
    /// handler path of `/xyz` results in a final path of `/abc/xyz`.
    fn path(&self) -> Option<String> {
        None
    }

    /// Optional list of server names for which a given controller should be registered.
    fn server_names(&self) -> Option<ServerNameSet> {
        None
    }

    /// Configures a [Router] to handle incoming requests. Passed instance ptr points to the
    /// controller component being processed (`Self`).
    fn configure_router(
        &self,
        router: Router,
        self_instance_ptr: ComponentInstancePtr<dyn Controller + Send + Sync>,
    ) -> Result<Router, ErrorPtr>;

    /// Creates a [Router] which is then passed to `configure_router`.
    fn create_router(&self) -> Result<Router, ErrorPtr>;

    /// Adds any post-route configuration to the [Router].
    fn post_configure_router(&self, router: Router) -> Result<Router, ErrorPtr>;
}

downcast_sync!(dyn Controller + Send + Sync);
