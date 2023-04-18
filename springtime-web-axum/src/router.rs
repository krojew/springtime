//! Controller routing handling. By default, routing is based on gathering existing controllers and
//! their request handlers.

use axum::Router;
use springtime_di::component_registry::conditional::unregistered_component;
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, injectable, Component};

/// Trait for creating a [Router], usually based on injected
/// [Controller](crate::controller::Controller)s.
#[injectable]
pub trait RouterBootstrap {
    /// Creates a new [Router].
    fn bootstrap_router(&self, server_name: &str) -> Result<Router, ErrorPtr>;
}

#[derive(Component)]
#[component(priority = -128, condition = "unregistered_component::<dyn RouterBootstrap + Send + Sync>")]
struct ControllerRouterBootstrap;

#[component_alias]
impl RouterBootstrap for ControllerRouterBootstrap {
    fn bootstrap_router(&self, _server_name: &str) -> Result<Router, ErrorPtr> {
        todo!()
    }
}
