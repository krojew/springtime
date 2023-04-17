//! Functionality related to defining [Controller]s.

use springtime_di::injectable;

/// Main trait for [Components](springtime_di::component::Component) used as controllers -
/// collections of web [handlers](axum::handler::Handler) being functions contained in typical
/// structs. Such approach allows for injecting other components via dependency injection, and
/// therefore, creating advanced applications with proper architecture.
#[injectable]
pub trait Controller {
    /// Prefix for all paths contained in the controller, e.g. controller path of `/abc` and handler
    /// path of `/xyz` results in final path of `/abc/xyz`.
    fn path(&self) -> Option<String> {
        None
    }
}
