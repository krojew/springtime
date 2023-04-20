use axum::Router;
use once_cell::sync::Lazy;
use portpicker::{pick_unused_port, Port};
use springtime::application;
use springtime::future::{BoxFuture, FutureExt};
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, Component};
use springtime_web_axum::axum::extract::Path;
use springtime_web_axum::config::{ServerConfig, WebConfig, WebConfigProvider};
use springtime_web_axum::controller;
use springtime_web_axum::server::{ShutdownSignalSender, ShutdownSignalSource};
use std::sync::Mutex;
use tokio::sync::Barrier;

#[derive(Component)]
struct TestController;

#[controller(path = "/test", server_names = ["default", "test"])]
impl TestController {
    #[get("/:user_id")]
    async fn hello_world(&self, Path(user_id): Path<u32>) -> String {
        format!("Hello {user_id}!")
    }

    #[post("/")]
    async fn post_something(&self) -> &'static str {
        "Posted!"
    }

    #[fallback]
    async fn fallback(&self) -> &'static str {
        "fallback"
    }

    #[router_source]
    fn create_router(&self) -> Result<Router, ErrorPtr> {
        Ok(Router::new())
    }

    #[router_post_configure]
    fn post_configure_router(&self, router: Router) -> Result<Router, ErrorPtr> {
        Ok(router)
    }
}

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
            server_config.listen_address = format!("127.0.0.1:{}", *PORT);

            let mut config = WebConfig::default();
            config.servers = [("test".to_string(), server_config)].into_iter().collect();

            Ok(Self { config })
        }
        .boxed()
    }
}

static SHUTDOWN_SIGNAL: Lazy<Mutex<Option<ShutdownSignalSender>>> = Lazy::new(Default::default);
static START_BARRIER: Lazy<Barrier> = Lazy::new(|| Barrier::new(2));
static PORT: Lazy<Port> = Lazy::new(|| pick_unused_port().unwrap());

#[derive(Component)]
struct TestShutdownSignalSource;

#[component_alias]
impl ShutdownSignalSource for TestShutdownSignalSource {
    fn register_shutdown(&self, shutdown_sender: ShutdownSignalSender) -> Result<(), ErrorPtr> {
        SHUTDOWN_SIGNAL.lock().unwrap().replace(shutdown_sender);
        tokio::spawn(async {
            START_BARRIER.wait().await;
        });

        Ok(())
    }
}

#[tokio::test]
async fn should_register_controller() {
    let handle = tokio::spawn(async {
        let mut application = application::create_default().unwrap();
        application.run().await.unwrap();
    });

    let body = reqwest::get(format!("http://localhost:{}/test/42", *PORT))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(body, "Hello 42!");

    let body = reqwest::get(format!("http://localhost:{}/test/invalid/route", *PORT))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(body, "fallback");

    START_BARRIER.wait().await;
    SHUTDOWN_SIGNAL
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .send(())
        .unwrap();

    handle.await.unwrap();
}
