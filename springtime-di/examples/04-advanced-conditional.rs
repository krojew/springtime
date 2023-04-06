// note: this example assumes you've analyzed the previous one

use springtime_di::component_registry::conditional::unregistered_component;
use springtime_di::factory::ComponentFactoryBuilder;
use springtime_di::instance_provider::{ComponentInstancePtr, TypedComponentInstanceProvider};
use springtime_di::{component_alias, injectable, Component};

#[injectable]
trait TestTrait {
    fn foo(&self);
}

#[derive(Component)]
struct TestDependency1;

#[component_alias]
impl TestTrait for TestDependency1 {
    fn foo(&self) {
        println!("Hello world from 1!");
    }
}

#[derive(Component)]
// a useful pattern is providing default components, if no other of given type are registered
// built-in "unregistered_component" condition can be used along with proper priority
// priority defines the order of registration of components with conditions (default: 0)
#[component(condition = "unregistered_component::<dyn TestTrait + Send + Sync>", priority = -100)]
struct TestDependency2;

#[component_alias]
impl TestTrait for TestDependency2 {
    fn foo(&self) {
        println!("Hello world from 2!");
    }
}

#[derive(Component)]
struct TestComponent {
    // TestDependency2 was disabled, because TestDependency1 got registered first, so there is no
    // ambiguity
    dependency: ComponentInstancePtr<dyn TestTrait + Send + Sync>,
}

impl TestComponent {
    fn call_foo(&self) {
        self.dependency.foo();
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

    // prints "Hello world from 1!"
    component.call_foo();
}
