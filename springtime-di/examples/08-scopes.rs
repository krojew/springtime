// note: this example assumes you've analyzed the previous one

use springtime_di::factory::ComponentFactoryBuilder;
use springtime_di::instance_provider::{
    ComponentInstancePtr, ErrorPtr, TypedComponentInstanceProvider,
};
use springtime_di::{component_alias, injectable, Component};
use std::sync::Mutex;

#[injectable]
trait TestTrait {
    fn foo(&self);
}

#[derive(Component)]
// scopes are containers for component instances and decide when to create and when to reuse
// instances
// "PROTOTYPE" is a built-in scope which creates a new instance on every request which is useful for
// stateful components; please see the scope module docs for more information
#[component(constructor = "TestDependency::new", scope = "PROTOTYPE")]
struct TestDependency {
    #[component(ignore)]
    // this is some example state which is not shared between other instances of this component
    some_state: Mutex<i32>,
}

#[component_alias]
impl TestTrait for TestDependency {
    fn foo(&self) {
        let mut some_state = self.some_state.lock().unwrap();
        *some_state += 1;

        println!("{}", some_state);
    }
}

impl TestDependency {
    fn new() -> Result<Self, ErrorPtr> {
        // to show we're constructed on each request, let's print some info
        println!("TestDependency created!");
        Ok(Self {
            some_state: Mutex::default(),
        })
    }
}

#[derive(Component)]
struct TestComponent {
    dependency_1: ComponentInstancePtr<dyn TestTrait + Send + Sync>,
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

    // prints "TestDependency created!" "TestDependency created!"
    let component = component_factory
        .primary_instance_typed::<TestComponent>()
        .expect("error creating TestComponent");

    // prints "1" "1" instead of "1" "2"
    component.call_foo();
}
