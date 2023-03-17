use springtime_di::component::{Component, ComponentInstanceProvider, ComponentInstancePtr};
use springtime_di::Error;
use springtime_di_derive::Component;
use std::any::TypeId;

#[derive(Component)]
struct TestDependency;

#[derive(Component)]
struct TestComponent1 {
    _dependency: ComponentInstancePtr<TestDependency>,
}

#[derive(Component)]
struct TestComponent2(ComponentInstancePtr<TestDependency>);

struct TestDependencyInstanceProvider;

impl ComponentInstanceProvider for TestDependencyInstanceProvider {
    fn primary_instance<T: Component + 'static>(&self) -> Result<ComponentInstancePtr<T>, Error> {
        if TypeId::of::<T>() == TypeId::of::<TestDependency>() {
            return T::create(self);
        }

        Err(Error::NoPrimaryInstance("TestDependency".into()))
    }
}

#[test]
fn should_directly_create_with_explicit_dependency() {
    let instance_provider = TestDependencyInstanceProvider;
    assert!(TestComponent1::create(&instance_provider).is_ok());
    assert!(TestComponent2::create(&instance_provider).is_ok());
}
