use portpicker::pick_unused_port;
use springtime::application;
use springtime_di::future::{BoxFuture, FutureExt};
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, Component};
use springtime_web_axum::config::{ServerConfig, WebConfig, WebConfigProvider};
use springtime_web_axum::controller;

#[derive(Component)]
struct TestController;

#[controller(path = "/test")]
impl TestController {}

#[derive(Component)]
#[component(constructor = "TestWebConfigProvider::new")]
struct TestWebConfigProvider {
    #[component(ignore)]
    config: WebConfig,
}

#[component_alias]
impl WebConfigProvider for TestWebConfigProvider {
    fn config(&self) -> BoxFuture<'_, Result<&WebConfig, ErrorPtr>> {
        async { Ok(&self.config) }.boxed()
    }
}

impl TestWebConfigProvider {
    fn new() -> BoxFuture<'static, Result<Self, ErrorPtr>> {
        async {
            let mut server_config = ServerConfig::default();
            server_config.listen_address = format!("127.0.0.1:{}", pick_unused_port().unwrap());

            let mut config = WebConfig::default();
            config.servers = [("test".to_string(), server_config)].into_iter().collect();

            Ok(Self { config })
        }
        .boxed()
    }
}

#[tokio::test]
async fn should_register_controller() {
    let mut application = application::create_default().unwrap();
    application.run().await.unwrap();
}
