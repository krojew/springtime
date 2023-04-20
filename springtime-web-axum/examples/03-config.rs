// note: this example assumes you've analyzed the previous one

use springtime::application;
use springtime::future::{BoxFuture, FutureExt};
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, Component};
use springtime_web_axum::config::{
    ServerConfig, WebConfig, WebConfigProvider, DEFAULT_SERVER_NAME,
};
use springtime_web_axum::controller;

// web is provided by a WebConfigProvider, which by default, uses a configuration file (see module
// documentation)
// to provide your own, register a component implementing this trait, and it should take precedence
// over the default one
#[derive(Component)]
#[component(constructor = "MyWebConfigProvider::new")]
struct MyWebConfigProvider {
    // this is the cached custom config
    #[component(ignore)]
    config: WebConfig,
}

impl MyWebConfigProvider {
    fn new() -> BoxFuture<'static, Result<Self, ErrorPtr>> {
        async {
            // start with a default web config and override what's needed
            let mut web_config = WebConfig::default();

            // start with a default server config and override what's needed
            let mut server_config = ServerConfig::default();

            // listen only on localhost interface
            server_config.listen_address = "127.0.0.1:80".to_string();

            // override default server configuration
            web_config
                .servers
                .insert(DEFAULT_SERVER_NAME.to_string(), server_config);

            Ok(Self { config: web_config })
        }
        .boxed()
    }
}

//noinspection DuplicatedCode
// register MyWebConfigProvider as a WebConfigProvider
#[component_alias]
impl WebConfigProvider for MyWebConfigProvider {
    fn config(&self) -> BoxFuture<'_, Result<&WebConfig, ErrorPtr>> {
        async { Ok(&self.config) }.boxed()
    }
}

#[derive(Component)]
struct ExampleController;

#[controller]
impl ExampleController {
    #[get("/")]
    async fn hello_world(&self) -> &'static str {
        "Hello world!"
    }
}

#[tokio::main]
async fn main() {
    let mut application = application::create_default().expect("unable to create application");
    application.run().await.expect("error running application");
}
