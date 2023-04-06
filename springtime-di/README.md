# Springtime Dependency Injection

[![crates.io version](https://img.shields.io/crates/v/springtime-di.svg)](https://crates.io/crates/springtime-di) 
![build status](https://github.com/krojew/springtime/actions/workflows/rust.yml/badge.svg) 
![Maintenance](https://img.shields.io/maintenance/yes/2023)

A dependency injection crate inspired by the [Spring Framework](https://spring.io/) in Java.

The philosophy of *Springtime* is to provide the means of easy dependency injection without unnecessary manual  
configuration, e.g. without the need to explicitly create dependencies and  storing them in containers. As much work
as possible is placed on compile-time metadata creation  and automatic component discovery, thus allowing users to focus 
on the _usage_ of components, rather than their _creation_ and _management_. With an accent placed on attributes, 
dependency configuration becomes declarative (_what I want to accomplish_) leaving the gritty details the framework
itself (_how to accomplish what was requested_).

## Features

* Concrete and trait object injection
* Automatic and manual registration support
* Component filtering
* Conditional component registration
* Component priorities
* Custom constructor functions
* Per-field configurable initialization

## Basic usage

*Springtime* is highly configurable, but the most basic usage example is quite simple and consists of using a few
attributes to fully configure the dependency chain. For tutorial, advanced features, and patterns, please look at the
examples, which form a step-by-step guide.

```rust
use springtime_di::factory::ComponentFactoryBuilder;
use springtime_di::instance_provider::{ComponentInstancePtr, TypedComponentInstanceProvider};
use springtime_di::{component_alias, injectable, Component};

// this is a trait we would like to use in our component
#[injectable]
trait TestTrait {
    fn foo(&self);
}

// this is a dependency which implements the above trait and also is an injectable component
#[derive(Component)]
struct TestDependency;

// we're telling the framework to provide TestDependency when asked for dyn TestTrait
#[component_alias]
impl TestTrait for TestDependency {
    fn foo(&self) {
        println!("Hello world!");
    }
}

// this is another component, but with a dependency
#[derive(Component)]
struct TestComponent {
    // the framework will know how to inject dyn TestTrait, when asked for TestComponent
    // more details are available in the documentation
    dependency: ComponentInstancePtr<dyn TestTrait + Send + Sync>,
    // alternatively, you can inject the concrete type
    // dependency: ComponentInstancePtr<TestDependency>,
}

impl TestComponent {
    fn call_foo(&self) {
        self.dependency.foo();
    }
}

// note: for the sake of simplicity, errors are unwrapped, rather than gracefully handled
fn main() {
    // components are created by a ComponentFactory
    // for convenience, ComponentFactoryBuilder can be used to create the factory with a reasonable
    // default configuration
    let mut component_factory = ComponentFactoryBuilder::new()
        .expect("error initializing ComponentFactoryBuilder")
        .build();

    let component = component_factory
        .primary_instance_typed::<TestComponent>()
        .expect("error creating TestComponent");

    // prints "Hello world!"
    component.call_foo();
}

```
