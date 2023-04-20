// note: this example assumes you've analyzed the previous one

use springtime::application;
use springtime::future::{BoxFuture, FutureExt};
use springtime::runner::ApplicationRunner;
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, Component};

#[derive(Component)]
struct PrintHelloRunner;

#[component_alias]
impl ApplicationRunner for PrintHelloRunner {
    fn run(&self) -> BoxFuture<'_, Result<(), ErrorPtr>> {
        async {
            print!("Hello ");
            Ok(())
        }
        .boxed()
    }

    // for ordered execution of application runners, priorities can be used
    fn priority(&self) -> i8 {
        3
    }
}

#[derive(Component)]
struct PrintWorldRunner;

#[component_alias]
impl ApplicationRunner for PrintWorldRunner {
    fn run(&self) -> BoxFuture<'_, Result<(), ErrorPtr>> {
        async {
            print!("world");
            Ok(())
        }
        .boxed()
    }

    // for ordered execution of application runners, priorities can be used
    fn priority(&self) -> i8 {
        2
    }
}

#[derive(Component)]
struct PrintExclamationRunner;

#[component_alias]
impl ApplicationRunner for PrintExclamationRunner {
    fn run(&self) -> BoxFuture<'_, Result<(), ErrorPtr>> {
        async {
            println!("!");
            Ok(())
        }
        .boxed()
    }

    // for ordered execution of application runners, priorities can be used
    fn priority(&self) -> i8 {
        1
    }
}

// note: for the sake of simplicity, errors are unwrapped, rather than gracefully handled
#[tokio::main]
async fn main() {
    let mut application =
        application::create_default().expect("unable to create default application");

    // prints "Hello world!"
    application.run().await.expect("error running application");
}
