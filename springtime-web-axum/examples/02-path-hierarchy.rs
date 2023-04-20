// note: this example assumes you've analyzed the previous one

use springtime::application;
use springtime_di::Component;
use springtime_web_axum_derive::controller;

#[derive(Component)]
struct ExampleController;

// set path prefix for all routes in this controller
#[controller(path = "/test")]
impl ExampleController {
    // this function will respond to GET request for http://localhost/test (or any network
    // interface)
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
