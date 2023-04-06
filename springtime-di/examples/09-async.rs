// note: this example assumes you've analyzed the previous one

use springtime_di::factory::ComponentFactoryBuilder;
use springtime_di::instance_provider::{ComponentInstancePtr, TypedComponentInstanceProvider};
use springtime_di::{component_alias, injectable, Component};

#[injectable]
trait TestTrait {
    fn foo(&self);
}

#[derive(Component)]
struct TestDependency;

#[component_alias]
impl TestTrait for TestDependency {
    fn foo(&self) {
        println!("Hello world!");
    }
}

#[derive(Component)]
struct TestComponent {
    dependency: ComponentInstancePtr<dyn TestTrait + Send + Sync>,
}

impl TestComponent {
    fn call_foo(&self) {
        self.dependency.foo();
    }
}

#[tokio::main]
async fn main() {
    let mut component_factory = ComponentFactoryBuilder::new()
        .expect("error initializing ComponentFactoryBuilder")
        .build();

    // with the "async" feature all construction/retrieval functions become async
    let component = component_factory
        .primary_instance_typed::<TestComponent>()
        .await
        .expect("error creating TestComponent");

    component.call_foo();
}
