use springtime_di::component::{Component, ComponentInstanceProvider, ComponentInstancePtr};
use springtime_di::component_registry::{
    ComponentDefinitionRegistry, StaticComponentDefinitionRegistry,
};
use springtime_di::error::ComponentInstanceProviderError;
use springtime_di::Component;
use std::any::TypeId;

#[derive(Component)]
struct TestDependency;

#[derive(Component)]
struct TestComponent1 {
    _dependency: ComponentInstancePtr<TestDependency>,
    #[component(default)]
    _default: i8,
    #[component(default = "dummy_expr")]
    _default_expr: i8,
}

#[derive(Component)]
#[component(name = "dep2", primary)]
struct TestComponent2(
    ComponentInstancePtr<TestDependency>,
    #[component(default = "dummy_expr")] i8,
);

fn dummy_expr() -> i8 {
    -1
}

struct TestDependencyInstanceProvider;

impl ComponentInstanceProvider for TestDependencyInstanceProvider {
    fn primary_instance<T: Component + 'static>(
        &self,
    ) -> Result<ComponentInstancePtr<T>, ComponentInstanceProviderError> {
        if TypeId::of::<T>() == TypeId::of::<TestDependency>() {
            return T::create(self).map(ComponentInstancePtr::new);
        }

        Err(ComponentInstanceProviderError::NoPrimaryInstance(
            "TestDependency".into(),
        ))
    }
}

#[test]
fn should_directly_create_with_explicit_dependency() {
    let instance_provider = TestDependencyInstanceProvider;
    assert!(TestComponent1::create(&instance_provider).is_ok());
    assert!(TestComponent2::create(&instance_provider).is_ok());
}

#[test]
fn should_register_components() {
    let registry = StaticComponentDefinitionRegistry::new(false).unwrap();
    assert!(registry.components_by_type::<TestDependency>().is_some());
    assert!(registry.components_by_type::<TestComponent2>().is_some());
}
