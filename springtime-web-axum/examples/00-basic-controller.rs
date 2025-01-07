use axum::extract::Path;
use springtime::application;
use springtime_di::Component;
use springtime_web_axum::controller;

// create a struct which will serve as our Controller - this implies it needs to be a Component for
// the dependency injection to work
#[derive(Component)]
struct ExampleController;

// mark the struct as a Controller - this will scan all functions for the controller attributes and
// create axum handlers out of them
#[controller]
impl ExampleController {
    // this function will respond to GET request for http://localhost/ (or any network interface)
    #[get("/")]
    async fn hello_world(&self) -> &'static str {
        "Hello world!"
    }

    // all axum features are available for controllers
    #[get("/{user}")]
    async fn hello_user(&self, Path(user): Path<String>) -> String {
        format!("Hello {user}!")
    }
}

// note: for the sake of simplicity, errors are unwrapped, rather than gracefully handled
#[tokio::main]
async fn main() {
    let mut application = application::create_default().expect("unable to create application");

    // run our server with default configuration - requests should be forwarded to ExampleController
    application.run().await.expect("error running application");
}
