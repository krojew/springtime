//! Functionality related to defining [Controller]s.

use springtime_di::injectable;

/// Main trait for [Components](springtime_di::component::Component) used as controllers -
/// collections of web [handlers](axum::handler::Handler) being functions contained in typical
/// structs. Such approach allows for injecting other components via dependency injection, and
/// therefore, creating advanced applications with proper architecture.
#[injectable]
pub trait Controller {}
