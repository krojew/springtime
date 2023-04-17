use springtime::application;
use springtime_di::Component;
use springtime_web_axum::controller;

#[derive(Component)]
struct TestController;

#[controller(path = "/test")]
impl TestController {}

#[tokio::test]
async fn should_register_controller() {
    let mut application = application::create_default().unwrap();
    application.run().await.unwrap();
}
