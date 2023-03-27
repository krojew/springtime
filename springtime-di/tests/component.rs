#[cfg(feature = "derive")]
mod component_test {
    use springtime_di::component::Component;
    use springtime_di::component_registry::{
        ComponentDefinitionRegistry, StaticComponentDefinitionRegistry,
    };
    use springtime_di::error::ComponentInstanceProviderError;
    use springtime_di::instance_provider::{
        ComponentInstanceAnyPtr, ComponentInstanceProvider, ComponentInstancePtr,
    };
    use springtime_di::{component_alias, Component};
    use std::any::TypeId;

    trait TestTrait1 {}

    trait TestTrait2 {}

    trait TestTrait3 {}

    #[derive(Component)]
    struct TestDependency;

    #[component_alias]
    impl TestTrait3 for TestDependency {}

    #[derive(Component)]
    struct TestComponent1 {
        _dependency_1: ComponentInstancePtr<TestDependency>,
        #[cfg(feature = "threadsafe")]
        _dependency_2: ComponentInstancePtr<dyn TestTrait3 + Sync + Send>,
        #[cfg(not(feature = "threadsafe"))]
        _dependency_2: ComponentInstancePtr<dyn TestTrait3>,
        _optional_dependency: Option<ComponentInstancePtr<TestDependency>>,
        #[component(default)]
        _default: i8,
        #[component(default = "dummy_expr")]
        _default_expr: i8,
        _all_deps: Vec<ComponentInstancePtr<dyn TestTrait3 + Sync + Send>>,
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
            #[cfg(feature = "threadsafe")]
            let trait_type = TypeId::of::<dyn TestTrait3 + Sync + Send>();
            #[cfg(not(feature = "threadsafe"))]
            let trait_type = TypeId::of::<dyn TestTrait3>();

            if type_id == trait_type || type_id == TypeId::of::<TestDependency>() {
                return TestDependency::create(self)
                    .map(|p| ComponentInstancePtr::new(p) as ComponentInstanceAnyPtr);
            }

            Err(ComponentInstanceProviderError::NoPrimaryInstance(type_id))
        }

        fn instances(
            &self,
            type_id: TypeId,
        ) -> Result<Vec<ComponentInstanceAnyPtr>, ComponentInstanceProviderError> {
            self.primary_instance(type_id).map(|p| vec![p])
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

        #[cfg(feature = "threadsafe")]
        assert!(registry
            .components_by_type::<dyn TestTrait1 + Sync + Send>()
            .is_some());
        #[cfg(feature = "threadsafe")]
        assert!(registry
            .components_by_type::<dyn TestTrait2 + Sync + Send>()
            .is_some());

        #[cfg(not(feature = "threadsafe"))]
        assert!(registry.components_by_type::<dyn TestTrait1>().is_some());
        #[cfg(not(feature = "threadsafe"))]
        assert!(registry.components_by_type::<dyn TestTrait2>().is_some());
    }
}
