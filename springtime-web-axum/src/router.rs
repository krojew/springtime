//! Controller routing handling. By default, routing is based on gathering existing controllers and
//! their request handlers.

use crate::controller::Controller;
use axum::Router;
use springtime_di::component_registry::conditional::unregistered_component;
use springtime_di::instance_provider::{ComponentInstancePtr, ErrorPtr};
use springtime_di::{component_alias, injectable, Component};
use std::sync::Arc;

/// Trait for creating a [Router], usually based on injected
/// [Controller](crate::controller::Controller)s.
#[injectable]
pub trait RouterBootstrap {
    /// Creates a new [Router].
    fn bootstrap_router(&self, server_name: &str) -> Result<Router, ErrorPtr>;
}

#[derive(Component)]
#[component(priority = -128, condition = "unregistered_component::<dyn RouterBootstrap + Send + Sync>")]
struct ControllerRouterBootstrap {
    controllers: Vec<ComponentInstancePtr<dyn Controller + Send + Sync>>,
}

#[component_alias]
impl RouterBootstrap for ControllerRouterBootstrap {
    fn bootstrap_router(&self, server_name: &str) -> Result<Router, ErrorPtr> {
        self.controllers
            .iter()
            .filter(|controller| {
                controller
                    .server_names()
                    .map(|server_names| server_names.contains(server_name))
                    .unwrap_or(true)
            })
            .try_fold(Router::new(), |router, controller| {
                let path = controller.path().unwrap_or_else(|| "/".to_string());
                controller
                    .configure_router(controller.clone())
                    .map_err(|error| Arc::new(error) as ErrorPtr)
                    .map(|inner_router| router.nest(&path, inner_router))
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::controller::MockController;
    use crate::router::{ControllerRouterBootstrap, RouterBootstrap};
    use axum::Router;
    use fxhash::FxHashSet;
    use springtime_di::instance_provider::ComponentInstancePtr;

    #[test]
    fn should_configure_router_with_filtering() {
        let mut controller = MockController::new();
        controller
            .expect_configure_router()
            .times(1)
            .return_const(Ok(Router::new()));
        controller.expect_server_names().times(1).return_const(
            ["1".to_string(), "2".to_string()]
                .into_iter()
                .collect::<FxHashSet<_>>(),
        );
        controller.expect_path().return_const(None);

        let bootstrap = ControllerRouterBootstrap {
            controllers: vec![ComponentInstancePtr::new(controller)],
        };
        assert!(bootstrap.bootstrap_router("1").is_ok());
    }

    #[test]
    fn should_not_configure_router_with_filtering() {
        let mut controller = MockController::new();
        controller
            .expect_configure_router()
            .times(0)
            .return_const(Ok(Router::new()));
        controller.expect_server_names().times(1).return_const(
            ["1".to_string(), "2".to_string()]
                .into_iter()
                .collect::<FxHashSet<_>>(),
        );

        let bootstrap = ControllerRouterBootstrap {
            controllers: vec![ComponentInstancePtr::new(controller)],
        };
        assert!(bootstrap.bootstrap_router("3").is_ok());
    }
}
