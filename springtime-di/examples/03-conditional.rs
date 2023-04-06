// note: this example assumes you've analyzed the previous one

use springtime_di::component_registry::conditional::{ConditionMetadata, Context};
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
// sometimes components may want to be conditionally registered, based on some runtime logic
// in such cases, condition expressions to call can be specified on components
#[component(condition = "can_test_dependency_2_be_registered")]
struct TestDependency2;

fn can_test_dependency_2_be_registered(
    _context: &dyn Context,
    _metadata: ConditionMetadata,
) -> bool {
    // for the sake of example, let's simply disable registration for this component
    false
}

#[component_alias]
impl TestTrait for TestDependency2 {
    fn foo(&self) {
        println!("Hello world from 2!");
    }
}

#[derive(Component)]
struct TestComponent {
    // given one of the above components has been conditionally disabled, there is no ambiguity in
    // what to inject
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
