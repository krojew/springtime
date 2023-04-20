// note: this example assumes you've analyzed the previous one

use springtime::application;
use springtime::runner::{BoxFuture, FutureExt};
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, Component};
use springtime_web_axum::config::{ServerConfig, WebConfig, WebConfigProvider};
use springtime_web_axum_derive::controller;

// the easiest way to create multiple server instances is to use the configuration file, but for the
// sake of example, a custom config provider is used here
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
            let mut server_1_config = ServerConfig::default();

            // listen only on localhost interface, port 80
            server_1_config.listen_address = "127.0.0.1:80".to_string();

            let mut server_2_config = ServerConfig::default();

            // listen only on localhost interface, port 8080
            server_2_config.listen_address = "127.0.0.1:8080".to_string();

            // define 2 server instances
            web_config.servers = [
                ("1".to_string(), server_1_config),
                ("2".to_string(), server_2_config),
            ]
            .into_iter()
            .collect();

            Ok(Self { config: web_config })
        }
        .boxed()
    }
}

//noinspection DuplicatedCode
#[component_alias]
impl WebConfigProvider for MyWebConfigProvider {
    fn config(&self) -> BoxFuture<'_, Result<&WebConfig, ErrorPtr>> {
        async { Ok(&self.config) }.boxed()
    }
}

#[derive(Component)]
struct ExampleController1;

// assign controller to server "1"
#[controller(server_names = ["1"])]
impl ExampleController1 {
    #[get("/")]
    async fn hello_world(&self) -> &'static str {
        "Hello world 1!"
    }
}

#[derive(Component)]
struct ExampleController2;

// assign controller to server "2"
#[controller(server_names = ["2"])]
impl ExampleController2 {
    // note the same path as the previous one
    #[get("/")]
    async fn hello_world(&self) -> &'static str {
        "Hello world 2!"
    }
}

#[tokio::main]
async fn main() {
    let mut application = application::create_default().expect("unable to create application");

    // http://localhost:80 will respond with "Hello world 1!"
    // http://localhost:8080 will respond with "Hello world 2!"
    application.run().await.expect("error running application");
}
