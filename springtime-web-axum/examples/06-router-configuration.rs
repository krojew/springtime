// note: this example assumes you've analyzed the previous one

use springtime::application;
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, Component};
use springtime_web_axum::axum::Router;
use springtime_web_axum::controller;
use springtime_web_axum::router::RouterConfigure;
use tower_http::compression::CompressionLayer;
use tower_http::validate_request::ValidateRequestHeaderLayer;

#[derive(Component)]
struct ExampleController;

#[controller]
impl ExampleController {
    #[get("/")]
    async fn hello_world(&self) -> &'static str {
        "Hello world!"
    }

    // axum Router can be configured in different ways
    // one is per-controller configuration by annotating a function with #[router_source]
    #[router_source]
    fn create_router(&self) -> Result<Router, ErrorPtr> {
        // a custom router can be created here, which then will be used to configure routes
        Ok(Router::new())
    }

    // the router can also be configured after all the routes have been added for given controller
    #[router_post_configure]
    fn post_configure_router(&self, router: Router) -> Result<Router, ErrorPtr> {
        Ok(router.route_layer(ValidateRequestHeaderLayer::bearer("password")))
    }
}

// another way to configure a router is by creating components implementing RouterConfigure
#[derive(Component)]
struct ExampleRouterConfigure;

#[component_alias]
impl RouterConfigure for ExampleRouterConfigure {
    fn configure(&self, router: Router) -> Result<Router, ErrorPtr> {
        // the router here is fully configured global one
        Ok(router.layer(CompressionLayer::new()))
    }
}

// yet another way is

#[tokio::main]
async fn main() {
    let mut application = application::create_default().expect("unable to create application");
    application.run().await.expect("error running application");
}
