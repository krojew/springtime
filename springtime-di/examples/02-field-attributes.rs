// note: this example assumes you've analyzed the previous one

use springtime_di::factory::ComponentFactoryBuilder;
use springtime_di::instance_provider::{ComponentInstancePtr, TypedComponentInstanceProvider};
use springtime_di::{component_alias, injectable, Component};

#[injectable]
trait TestTrait {
    fn foo(&self);
}

#[derive(Component)]
struct TestDependency {
    // some fields are not injectable, so they should be excluded from dependency injection
    // since all fields must be initialized, a custom expression to be called can be used to
    // initialize them
    #[component(default = "create_message")]
    message: String,

    // if necessary, a field can be initialized using Default::default()
    #[component(default)]
    _field_with_default: i8,
}

fn create_message() -> String {
    "Hello world!".to_string()
}

// we're telling the framework to provide TestDependency when asked for dyn TestTrait
#[component_alias]
impl TestTrait for TestDependency {
    fn foo(&self) {
        println!("{}", &self.message);
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
