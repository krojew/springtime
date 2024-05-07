# Springtime

[![crates.io version](https://img.shields.io/crates/v/springtime.svg)](https://crates.io/crates/springtime)
![build status](https://github.com/krojew/springtime/actions/workflows/rust.yml/badge.svg)

Application framework based on 
[springtime-di](https://crates.io/crates/springtime-di) dependency injection.
Inspired by the [Spring Framework](https://spring.io/) in Java, *Springtime*
provides a way to create advanced modular Rust applications by ensuring all
components of the application are properly decoupled from each other, and are
managed by the dependency injection system.

The core concept revolves around providing basic application services, e.g. 
logging, and running ordered `ApplicationRunner`s. An `ApplicationRunner`
represents root application service which starts the application logic. Examples
of runners are HTTP servers, messaging systems consumers, or even command line
applications. This crate provides the building blocks for more specialized 
crates which like to utilize *Springtime* to provide additional functionality,
e.g. web server runners.

## Features

* Automatic application logic discovery and running (based on DI)
* Runner priorities
* Configurable logging implementation (based on tracing)
* Async + sync support (runtime agnostic)

## Basic usage

*Springtime* is highly configurable, but the most basic usage example is quite
simple and consists of creating an `Application` instance and calling `run()`.
For **tutorial**, advanced features, and patterns, please look at the
[examples](https://github.com/krojew/springtime/tree/master/springtime/examples),
which form a step-by-step guide.

The following example assumes familiarity with [springtime-di](https://crates.io/crates/springtime-di)
and using the `async` feature.

```rust
// the following example shows how to inject an example HTTP server and run it

// this is an application runner, which will run when the application starts; the framework will
// automatically discover it using dependency injection
#[derive(Component)]
struct HttpRunner {
    // let the framework inject the example server
    http_server: ComponentInstancePtr<HttpServer>,
}

#[component_alias]
impl ApplicationRunner for HttpRunner {
    // note: BoxFuture is only needed when using the "async" feature
    fn run(&self) -> BoxFuture<'_, Result<(), ErrorPtr>> {
        // run the example server (run() is assumed to return a Future)
        self.http_server.run().boxed()
    }
}

// note: for the sake of simplicity, errors are unwrapped, rather than gracefully handled
#[tokio::main]
async fn main() {
    // create our application, which will detect all runners
    let mut application =
        application::create_default().expect("unable to create default application");

    // runs all ApplicationRunners, which means our HttpServer
    application.run().await.expect("error running application");
}
```
