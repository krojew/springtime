// note: this example assumes you've analyzed the previous one

use springtime_di::factory::ComponentFactoryBuilder;
use springtime_di::instance_provider::{
    ComponentInstancePtr, ErrorPtr, TypedComponentInstanceProvider,
};
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
        print!("Hello ");
    }
}

#[derive(Component)]
// sometimes there's a need to do some work during construction, which means bypassing the standard
// component creation process and invoking a custom constructor function
#[component(constructor = "TestComponent::new")]
struct TestComponent {
    // by default, fields are passed as arguments to the constructor, so dependencies can be
    // injected
    dependency: ComponentInstancePtr<dyn TestTrait + Send + Sync>,
    // some fields are not injectable dependencies, so they need to be excluded from the constructor
    #[component(ignore)]
    message: String,
}

impl TestComponent {
    // this will be called by the framework when TestComponent needs to be constructed
    // naturally, Option<> and Vec<> are also supported
    fn new(
        dependency: ComponentInstancePtr<dyn TestTrait + Send + Sync>,
    ) -> Result<Self, ErrorPtr> {
        Ok(Self {
            dependency,
            message: "world!".to_string(),
        })
    }
}

impl TestComponent {
    fn call_foo(&self) {
        self.dependency.foo();
        println!("{}", self.message);
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

    // prints "Hello world!"
    component.call_foo();
}
