# Springtime Web Axum

[![crates.io version](https://img.shields.io/crates/v/springtime-web-axum.svg)](https://crates.io/crates/springtime-web-axum)
![build status](https://github.com/krojew/springtime/actions/workflows/rust.yml/badge.svg)
![Maintenance](https://img.shields.io/maintenance/yes/2023)

Web framework based on [Springtime](https://crates.io/crates/springtime)
application framework and [axum](https://crates.io/crates/axum). Inspired by the
[Spring Framework](https://spring.io/) in Java, *Springtime Web Axum* provides a
way to create advanced modular Rust web applications by ensuring all components
of the application are properly decoupled from each other, and are managed by
the dependency injection system.

While `axum` provides a way to explicitly create web handlers in an imperative
way, this crate gives the option to create multi-layer applications in a 
declarative way, leveraging underlying dependency injection. This enables rapid
application development and loose coupling between components.

## Features

* Automatic controller discovery (web handlers with dependency injection)
* Hierarchical router paths
* Multiple server instances support with controller filtering
* Built-in external file and programmable configuration
* All the features provided by `axum`

## Basic usage

*Springtime Web Axum* is highly configurable, but the most basic usage example
is quite simple and consists of declaring controllers, creating an `Application`
instance and calling `run()`. For **tutorial**, advanced features, and patterns,
please look at the [examples](https://github.com/krojew/springtime/tree/master/springtime-web-axum/examples),
which form a step-by-step guide.

The following example assumes familiarity with 
[springtime](https://crates.io/crates/springtime) and 
[springtime-di](https://crates.io/crates/springtime-di).

```rust
use axum::extract::Path;
use springtime::application;
use springtime_di::instance_provider::ComponentInstancePtr;
use springtime_di::{injectable, Component, component_alias};
use springtime_web_axum_derive::controller;

// injectable example trait representing a domain service
#[injectable]
trait DomainService {
    fn get_important_message(&self, user: &str) -> String;
}

// concrete service implementation
#[derive(Component)]
struct ExampleDomainService;

// register ExampleDomainService as providing dyn DomainService
#[component_alias]
impl DomainService for ExampleDomainService {
    fn get_important_message(&self, user: &str) -> String {
        format!("Hello {}!", user)
    }
}

// create a struct which will serve as our Controller - this implies it needs to be a Component for
// the dependency injection to work
#[derive(Component)]
struct ExampleController {
    // inject the domain service (alternatively, inject concrete type instead of a trait)
    service: ComponentInstancePtr<dyn DomainService + Send + Sync>,
}

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
    #[get("/:user")]
    async fn hello_user(&self, Path(user): Path<String>) -> String {
        // delegate work to our domain service
        self.service.get_important_message(&user)
    }
}

// note: for the sake of simplicity, errors are unwrapped, rather than gracefully handled
#[tokio::main]
async fn main() {
    let mut application =
        application::create_default().expect("unable to create application");

    // run our server with default configuration - requests should be forwarded to ExampleController
    application.run().await.expect("error running application");
}
```
