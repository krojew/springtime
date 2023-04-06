// note: this example assumes you've analyzed the previous one

use springtime_di::factory::ComponentFactoryBuilder;
use springtime_di::instance_provider::{ComponentInstancePtr, TypedComponentInstanceProvider};
use springtime_di::{component_alias, injectable, Component};

#[injectable]
trait TestTrait {
    fn compute_important_number(&self) -> i32;
}

#[derive(Component)]
struct TestDependency1;

#[component_alias]
impl TestTrait for TestDependency1 {
    fn compute_important_number(&self) -> i32 {
        9 * 3
    }
}

#[derive(Component)]
struct TestDependency2;

#[component_alias]
impl TestTrait for TestDependency2 {
    fn compute_important_number(&self) -> i32 {
        3 * 5
    }
}

#[derive(Component)]
// constructors not only accept dependencies generated from fields, but can also accept additional
// arguments; the syntax described in the "component" module docs
#[component(
    constructor = "TestComponent::new",
    constructor_parameters = "Vec<dyn TestTrait + Send + Sync>"
)]
struct TestComponent {
    #[component(ignore)]
    important_number: i32,
}

impl TestComponent {
    // a common pattern in constructors is to compute some state based on other (implicit)
    // dependencies
    fn new(dependencies: Vec<ComponentInstancePtr<dyn TestTrait + Send + Sync>>) -> Self {
        Self {
            important_number: dependencies
                .iter()
                .fold(0, |result, dep| result + dep.compute_important_number()),
        }
    }
}

impl TestComponent {
    fn call_foo(&self) {
        println!("The answer is {}", self.important_number);
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

    component.call_foo();
}
