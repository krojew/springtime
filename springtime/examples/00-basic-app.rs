use springtime::application;
use springtime::future::{BoxFuture, FutureExt};
use springtime::runner::ApplicationRunner;
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, Component};

// this is an application runner, which will run when the application starts; the framework will
// automatically discover it using dependency injection
#[derive(Component)]
struct HelloWorldRunner;

//noinspection DuplicatedCode
#[component_alias]
impl ApplicationRunner for HelloWorldRunner {
    // note: BoxFuture is only needed when using the "async" feature
    fn run(&self) -> BoxFuture<'_, Result<(), ErrorPtr>> {
        async {
            println!("Hello world!");
            Ok(())
        }
        .boxed()
    }
}

// note: for the sake of simplicity, errors are unwrapped, rather than gracefully handled
#[tokio::main]
async fn main() {
    // create our application, which will detect all runners
    let mut application =
        application::create_default().expect("unable to create default application");

    // prints "Hello world!"
    application.run().await.expect("error running application");
}
