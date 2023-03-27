use springtime_di::component::{
    Component, ComponentInstanceAnyPtr, ComponentInstanceProvider, ComponentInstancePtr,
};
use springtime_di::component_registry::{
    ComponentDefinitionRegistry, StaticComponentDefinitionRegistry,
};
use springtime_di::error::ComponentInstanceProviderError;
use springtime_di::{component_alias, Component};
use std::any::TypeId;

trait TestTrait1 {}

trait TestTrait2 {}

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
#[component(names = ["dep2"])]
struct TestComponent2(
    ComponentInstancePtr<TestDependency>,
    #[component(default = "dummy_expr")] i8,
);

#[component_alias]
impl TestTrait1 for TestComponent2 {}

#[component_alias(primary)]
impl TestTrait2 for TestComponent2 {}

fn dummy_expr() -> i8 {
    -1
}

struct TestDependencyInstanceProvider;

impl ComponentInstanceProvider for TestDependencyInstanceProvider {
    fn primary_instance(
        &self,
        type_id: TypeId,
    ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
        if type_id == TypeId::of::<TestDependency>() {
            return TestDependency::create(self)
                .map(|p| ComponentInstancePtr::new(p) as ComponentInstanceAnyPtr);
        }

        Err(ComponentInstanceProviderError::NoPrimaryInstance(type_id))
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
    assert!(registry.components_by_type::<dyn TestTrait1>().is_some());
    assert!(registry.components_by_type::<dyn TestTrait2>().is_some());
}
