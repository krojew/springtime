// note: this example assumes you've analyzed the previous one

use springtime::application;
use springtime_di::Component;
use springtime_web_axum::controller;

#[derive(Component)]
struct ExampleController;

#[controller]
impl ExampleController {
    #[get("/")]
    async fn hello_world(&self) -> &'static str {
        "Hello world!"
    }

    // register a fallback route when none match
    #[fallback]
    async fn fallback(&self) -> &'static str {
        "No route matched!"
    }
}

#[tokio::main]
async fn main() {
    let mut application = application::create_default().expect("unable to create application");
    application.run().await.expect("error running application");
}
