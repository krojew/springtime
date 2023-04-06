// note: this example assumes you've analyzed the previous one

use springtime_di::factory::ComponentFactoryBuilder;
use springtime_di::instance_provider::{ComponentInstancePtr, TypedComponentInstanceProvider};
use springtime_di::{component_alias, injectable, Component};

#[injectable]
trait TestTrait1 {
    fn foo(&self);
}

#[injectable]
trait TestTrait2 {
    fn foo(&self);
}

#[derive(Component)]
struct TestDependency;

// notice the lack of #[component_alias] making dyn TestTrait1 + Send + Sync for TestDependency not
// injectable
impl TestTrait1 for TestDependency {
    fn foo(&self) {
        println!("Hello world for 1!");
    }
}

#[component_alias]
impl TestTrait2 for TestDependency {
    fn foo(&self) {
        println!("Hello world for 2!");
    }
}

#[derive(Component)]
struct TestComponent {
    // no component registered itself with TestTrait1, so the dependency is not satisfied, but using
    // Option<> still allows TestComponent to be constructed
    dependency: Option<ComponentInstancePtr<dyn TestTrait1 + Send + Sync>>,
    // inject all registered components for dyn TestTrait2 + Send + Sync
    all_dependencies: Vec<ComponentInstancePtr<dyn TestTrait2 + Send + Sync>>,
}

impl TestComponent {
    fn call_foo(&self) {
        if let Some(dependency) = &self.dependency {
            dependency.foo();
        }

        for dependency in &self.all_dependencies {
            dependency.foo();
        }
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

    // prints "Hello world for 2!"
    component.call_foo();
}
