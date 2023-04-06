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

//noinspection DuplicatedCode
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
