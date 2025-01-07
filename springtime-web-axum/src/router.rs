//! Controller routing handling. By default, routing is based on gathering existing controllers and
//! their request handlers.

use crate::controller::Controller;
use axum::Router;
#[cfg(test)]
use mockall::automock;
use springtime_di::component_registry::conditional::unregistered_component;
use springtime_di::instance_provider::{ComponentInstancePtr, ErrorPtr};
use springtime_di::{component_alias, injectable, Component};
use tracing::debug;

/// Trait for configuring [Router] created by [RouterBootstrap]. Multiple such components can be
/// present and each one will be called with the current router instance.
#[injectable]
#[cfg_attr(test, automock)]
pub trait RouterConfigure {
    /// Configure and return existing [Router].
    fn configure(&self, router: Router) -> Result<Router, ErrorPtr>;
}

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
    configure_components: Vec<ComponentInstancePtr<dyn RouterConfigure + Send + Sync>>,
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
                let inner_router = controller.create_router()?;

                debug!(path, "Registering new controller routes.");

                controller
                    .configure_router(inner_router, controller.clone())
                    .and_then(|inner_router| controller.post_configure_router(inner_router))
                    .map(|inner_router| {
                        if path.is_empty() || path == "/" {
                            // cannot nest root-level routers
                            router.merge(inner_router)
                        } else {
                            router.nest(&path, inner_router)
                        }
                    })
            })
            .and_then(|router| {
                self.configure_components
                    .iter()
                    .try_fold(router, |router, configure| configure.configure(router))
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::controller::MockController;
    use crate::router::{ControllerRouterBootstrap, MockRouterConfigure, RouterBootstrap};
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
        controller
            .expect_create_router()
            .return_const(Ok(Router::new()));
        controller
            .expect_post_configure_router()
            .returning(|router| Ok(router));

        let bootstrap = ControllerRouterBootstrap {
            controllers: vec![ComponentInstancePtr::new(controller)],
            configure_components: vec![],
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
            configure_components: vec![],
        };
        assert!(bootstrap.bootstrap_router("3").is_ok());
    }

    #[test]
    fn should_pass_existing_router_for_configuration() {
        let mut configure = MockRouterConfigure::new();
        configure
            .expect_configure()
            .times(1)
            .returning(|router| Ok(router));

        let bootstrap = ControllerRouterBootstrap {
            controllers: vec![],
            configure_components: vec![ComponentInstancePtr::new(configure)],
        };
        assert!(bootstrap.bootstrap_router("1").is_ok());
    }
}
