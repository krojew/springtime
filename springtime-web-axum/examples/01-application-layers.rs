// note: this example assumes you've analyzed the previous one

use axum::extract::Path;
use springtime::application;
use springtime_di::instance_provider::ComponentInstancePtr;
use springtime_di::{component_alias, injectable, Component};
use springtime_web_axum::controller;

// injectable example trait representing a domain service
#[injectable]
trait DomainService {
    fn get_important_message(&self, user: &str) -> String;
}

// concrete service implementation
#[derive(Component)]
struct ExampleDomainService;

#[component_alias]
impl DomainService for ExampleDomainService {
    fn get_important_message(&self, user: &str) -> String {
        format!("Hello {user}!")
    }
}

#[derive(Component)]
struct ExampleController {
    // inject the domain service (alternatively, inject concrete type instead of a trait)
    service: ComponentInstancePtr<dyn DomainService + Send + Sync>,
}

#[controller]
impl ExampleController {
    #[get("/:user")]
    async fn hello_user(&self, Path(user): Path<String>) -> String {
        // delegate work to our domain service
        self.service.get_important_message(&user)
    }
}

#[tokio::main]
async fn main() {
    let mut application = application::create_default().expect("unable to create application");
    application.run().await.expect("error running application");
}
