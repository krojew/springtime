#[cfg(feature = "derive")]
mod component_derive_test {
    use springtime_di::component::{Component, ComponentDowncast};
    use springtime_di::component_registry::{
        ComponentDefinitionRegistry, StaticComponentDefinitionRegistry,
    };
    use springtime_di::error::ComponentInstanceProviderError;
    use springtime_di::instance_provider::{
        CastFunction, ComponentInstanceAnyPtr, ComponentInstanceProvider, ComponentInstancePtr,
    };
    use springtime_di::{component_alias, injectable, Component};
    use std::any::TypeId;

    #[injectable]
    trait TestTrait1 {}

    #[injectable]
    trait TestTrait2 {}

    #[injectable]
    trait TestTrait3 {}

    #[derive(Component)]
    struct TestDependency;

    #[component_alias]
    impl TestTrait2 for TestDependency {}

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
        #[cfg(feature = "threadsafe")]
        #[component(name = "test_dependency")]
        _named_dependency: ComponentInstancePtr<dyn TestTrait3 + Sync + Send>,
        #[cfg(not(feature = "threadsafe"))]
        #[component(name = "test_dependency")]
        _named_dependency: ComponentInstancePtr<dyn TestTrait3>,
        #[cfg(feature = "threadsafe")]
        #[component(name = "test_dependency")]
        _named_optional_dependency: Option<ComponentInstancePtr<dyn TestTrait3 + Sync + Send>>,
        #[cfg(not(feature = "threadsafe"))]
        #[component(name = "test_dependency")]
        _named_optional_dependency: Option<ComponentInstancePtr<dyn TestTrait3>>,
        #[component(default)]
        _default: i8,
        #[component(default = "dummy_expr")]
        _default_expr: i8,
        _all_dependencies: Vec<ComponentInstancePtr<dyn TestTrait3 + Sync + Send>>,
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

    unsafe fn cast_dependency(
        instance: ComponentInstanceAnyPtr,
        result: *mut (),
    ) -> Result<(), ComponentInstanceAnyPtr> {
        let p = TestDependency::downcast(instance)?;
        let result = &mut *(result as *mut Option<ComponentInstancePtr<TestDependency>>);
        result.replace(p);
        Ok(())
    }

    unsafe fn cast_trait(
        instance: ComponentInstanceAnyPtr,
        result: *mut (),
    ) -> Result<(), ComponentInstanceAnyPtr> {
        #[cfg(feature = "threadsafe")]
        {
            let p = <dyn TestTrait3 + Sync + Send as ComponentDowncast<TestDependency>>::downcast(
                instance,
            )?;
            let result =
                &mut *(result as *mut Option<ComponentInstancePtr<dyn TestTrait3 + Sync + Send>>);
            result.replace(p);
        }
        #[cfg(not(feature = "threadsafe"))]
        {
            let p = <dyn TestTrait3 as ComponentDowncast<TestDependency>>::downcast(instance)?;
            let result = &mut *(result as *mut Option<ComponentInstancePtr<dyn TestTrait3>>);
            result.replace(p);
        }

        Ok(())
    }

    struct TestDependencyInstanceProvider;

    impl ComponentInstanceProvider for TestDependencyInstanceProvider {
        fn primary_instance(
            &self,
            type_id: TypeId,
        ) -> Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError>
        {
            #[cfg(feature = "threadsafe")]
            let trait_type = TypeId::of::<dyn TestTrait3 + Sync + Send>();
            #[cfg(not(feature = "threadsafe"))]
            let trait_type = TypeId::of::<dyn TestTrait3>();

            if type_id == TypeId::of::<TestDependency>() {
                return TestDependency::create(self).map(|p| {
                    (
                        ComponentInstancePtr::new(p) as ComponentInstanceAnyPtr,
                        cast_dependency as CastFunction,
                    )
                });
            }

            if type_id == trait_type {
                return TestDependency::create(self).map(|p| {
                    (
                        ComponentInstancePtr::new(p) as ComponentInstanceAnyPtr,
                        cast_trait as CastFunction,
                    )
                });
            }

            Err(ComponentInstanceProviderError::NoPrimaryInstance(type_id))
        }

        fn instances(
            &self,
            type_id: TypeId,
        ) -> Result<(Vec<ComponentInstanceAnyPtr>, CastFunction), ComponentInstanceProviderError>
        {
            self.primary_instance(type_id)
                .map(|(p, cast)| (vec![p], cast))
        }

        fn instance_by_name(
            &self,
            type_id: TypeId,
            name: &str,
        ) -> Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError>
        {
            if name == "test_dependency" {
                self.primary_instance(type_id)
            } else {
                Err(ComponentInstanceProviderError::NoNamedInstance(
                    name.to_string(),
                ))
            }
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
