// note: this example assumes you've analyzed the previous one

use springtime_di::factory::ComponentFactoryBuilder;
use springtime_di::instance_provider::{ComponentInstancePtr, TypedComponentInstanceProvider};
use springtime_di::{component_alias, injectable, Component};

#[injectable]
trait TestTrait {
    fn foo(&self);
}

#[derive(Component)]
struct TestDependency1;

// if the same trait is implemented for multiple types, one can be marked as primary to allow the
// framework to inject a single instance (called a "primary instance")
#[component_alias(primary)]
impl TestTrait for TestDependency1 {
    fn foo(&self) {
        println!("Hello world from 1!");
    }
}

#[derive(Component)]
// components have names which can be referenced to inject a specific one; if not specified, a
// default one is generated from the struct name by converting it to snake_case
#[component(names = ["some_name"])]
struct TestDependency2;

#[component_alias]
impl TestTrait for TestDependency2 {
    fn foo(&self) {
        println!("Hello world from 2!");
    }
}

#[derive(Component)]
struct TestComponent {
    // since there are several candidates implementing dyn TestTrait, the framework needs to decide
    // which to inject - if no specification is present, the primary instance is injected
    dependency_1: ComponentInstancePtr<dyn TestTrait + Send + Sync>,
    // it's possible to manually specify which instance to inject
    #[component(name = "some_name")]
    dependency_2: ComponentInstancePtr<dyn TestTrait + Send + Sync>,
}

impl TestComponent {
    fn call_foo(&self) {
        self.dependency_1.foo();
        self.dependency_2.foo();
    }
}

//noinspection DuplicatedCode
fn main() {
    let mut component_factory = ComponentFactoryBuilder::new()
        .expect("error initializing ComponentFactoryBuilder")
        .build();

    let component = component_factory
        .primary_instance_typed::<TestComponent>()
        .expect("error creating TestComponent");

    // prints "Hello world from 1!" "Hello world from 2!"
    component.call_foo();
}
