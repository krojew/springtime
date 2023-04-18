//! Functionality related to defining [Controller]s.

use axum::Router;
use fxhash::FxHashSet;
#[cfg(test)]
use mockall::automock;
use springtime_di::injectable;

/// Main trait for [Components](springtime_di::component::Component) used as controllers -
/// collections of web [handlers](axum::handler::Handler) being functions contained in typical
/// structs. Such approach allows for injecting other components via dependency injection, and
/// therefore, creating advanced applications with proper architecture.
#[injectable]
#[cfg_attr(test, automock)]
pub trait Controller {
    /// Prefix for all paths contained in the controller, e.g. controller path of `/abc` and handler
    /// path of `/xyz` results in final path of `/abc/xyz`.
    fn path(&self) -> Option<String> {
        None
    }

    /// Optional list of server names for which given controller should be registered.
    fn server_names(&self) -> Option<FxHashSet<String>> {
        None
    }

    /// Configures given [Router] to handle incoming requests.
    fn configure_router(&self, router: Router) -> Router;
}
