#[cfg(feature = "derive")]
mod component_derive_test {
    use springtime_di::component::{Component, ComponentDowncast};
    use springtime_di::component_registry::conditional::{
        ConditionMetadata, Context, SimpleContextFactory,
    };
    use springtime_di::component_registry::{
        ComponentDefinitionRegistry, StaticComponentDefinitionRegistry,
        TypedComponentDefinitionRegistry,
    };
    use springtime_di::instance_provider::ComponentInstanceProviderError;
    use springtime_di::instance_provider::{
        CastFunction, ComponentInstanceAnyPtr, ComponentInstanceProvider, ComponentInstancePtr,
    };
    use springtime_di::{component_alias, injectable, Component};
    use std::any::{Any, TypeId};

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

    #[component_alias(
        condition = "springtime_di::component_registry::conditional::registered_component::<TestDependency>"
    )]
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
    #[component(names = ["dep2"], condition = "dummy_component_condition")]
    struct TestComponent2(
        ComponentInstancePtr<TestDependency>,
        #[component(default = "dummy_expr")] i8,
    );

    #[component_alias(name = "test_trait_1_alias_name")]
    impl TestTrait1 for TestComponent2 {}

    #[component_alias(primary, condition = "dummy_alias_condition", priority = 100)]
    impl TestTrait2 for TestComponent2 {}

    #[derive(Component)]
    #[cfg_attr(
        feature = "threadsafe",
        component(
            constructor = "test_component_3",
            scope_name = "PROTOTYPE",
            constructor_parameters = "TestComponent2,dyn TestTrait1 + Sync + Send/test_trait_1_alias_name,Vec<dyn TestTrait1 + Sync + Send>,Option<TestComponent2>"
        )
    )]
    #[cfg_attr(
        not(feature = "threadsafe"),
        component(
            constructor = "test_component_3",
            scope_name = "PROTOTYPE",
            constructor_parameters = "TestComponent2,dyn TestTrait1/test_trait_1_alias_name,Vec<dyn TestTrait1>,Option<TestComponent2>"
        )
    )]
    struct TestComponent3 {
        _dependency: ComponentInstancePtr<TestDependency>,
        #[component(ignore)]
        _ignored: i8,
    }

    #[component_alias(
        primary,
        condition = "springtime_di::component_registry::conditional::unregistered_component::<TestComponent2>"
    )]
    impl TestTrait2 for TestComponent3 {}

    #[cfg(feature = "threadsafe")]
    fn test_component_3(
        dependency: ComponentInstancePtr<TestDependency>,
        _: ComponentInstancePtr<TestComponent2>,
        _: ComponentInstancePtr<dyn TestTrait1 + Sync + Send>,
        _: Vec<ComponentInstancePtr<dyn TestTrait1 + Sync + Send>>,
        _: Option<ComponentInstancePtr<TestComponent2>>,
    ) -> TestComponent3 {
        TestComponent3 {
            _dependency: dependency,
            _ignored: 0,
        }
    }
    #[cfg(not(feature = "threadsafe"))]
    fn test_component_3(
        dependency: ComponentInstancePtr<TestDependency>,
        _: ComponentInstancePtr<TestComponent2>,
        _: ComponentInstancePtr<dyn TestTrait1>,
        _: Vec<ComponentInstancePtr<dyn TestTrait1>>,
        _: Option<ComponentInstancePtr<TestComponent2>>,
    ) -> TestComponent3 {
        TestComponent3 {
            _dependency: dependency,
            _ignored: 0,
        }
    }

    fn dummy_expr() -> i8 {
        -1
    }

    fn dummy_component_condition(_context: &dyn Context, _metadata: ConditionMetadata) -> bool {
        true
    }

    fn dummy_alias_condition(_context: &dyn Context, _metadata: ConditionMetadata) -> bool {
        true
    }

    fn cast_dependency(
        instance: ComponentInstanceAnyPtr,
    ) -> Result<Box<dyn Any>, ComponentInstanceAnyPtr> {
        TestDependency::downcast(instance).map(|p| Box::new(p) as Box<dyn Any>)
    }

    fn cast_trait(
        instance: ComponentInstanceAnyPtr,
    ) -> Result<Box<dyn Any>, ComponentInstanceAnyPtr> {
        #[cfg(feature = "threadsafe")]
        {
            <dyn TestTrait3 + Sync + Send as ComponentDowncast<TestDependency>>::downcast(instance)
                .map(|p| Box::new(p) as Box<dyn Any>)
        }
        #[cfg(not(feature = "threadsafe"))]
        {
            <dyn TestTrait3 as ComponentDowncast<TestDependency>>::downcast(instance)
                .map(|p| Box::new(p) as Box<dyn Any>)
        }
    }

    struct TestDependencyInstanceProvider;

    impl ComponentInstanceProvider for TestDependencyInstanceProvider {
        fn primary_instance(
            &mut self,
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
            &mut self,
            type_id: TypeId,
        ) -> Result<Vec<(ComponentInstanceAnyPtr, CastFunction)>, ComponentInstanceProviderError>
        {
            self.primary_instance(type_id)
                .map(|(p, cast)| vec![(p, cast)])
        }

        fn instance_by_name(
            &mut self,
            name: &str,
        ) -> Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError>
        {
            if name == "test_dependency" {
                #[cfg(feature = "threadsafe")]
                let trait_type = TypeId::of::<dyn TestTrait3 + Sync + Send>();
                #[cfg(not(feature = "threadsafe"))]
                let trait_type = TypeId::of::<dyn TestTrait3>();

                self.primary_instance(trait_type)
            } else {
                Err(ComponentInstanceProviderError::NoNamedInstance(
                    name.to_string(),
                ))
            }
        }
    }

    #[derive(Component)]
    #[component(condition = "disabled_condition")]
    struct DisabledComponent;

    fn disabled_condition(_context: &dyn Context, _metadata: ConditionMetadata) -> bool {
        false
    }

    #[test]
    fn should_not_register_disabled_component() {
        let registry =
            StaticComponentDefinitionRegistry::new(false, &SimpleContextFactory::default())
                .unwrap();
        assert!(!TypedComponentDefinitionRegistry::is_registered_typed::<
            DisabledComponent,
        >(&registry));
    }

    #[test]
    fn should_directly_create_with_explicit_dependency() {
        let mut instance_provider = TestDependencyInstanceProvider;
        assert!(TestComponent1::create(&mut instance_provider).is_ok());
        assert!(TestComponent2::create(&mut instance_provider).is_ok());
    }

    #[test]
    fn should_register_components() {
        let registry =
            StaticComponentDefinitionRegistry::new(false, &SimpleContextFactory::default())
                .unwrap();
        assert!(!registry
            .components_by_type_typed::<TestDependency>()
            .is_empty());
        assert!(!registry
            .components_by_type_typed::<TestComponent2>()
            .is_empty());

        #[cfg(feature = "threadsafe")]
        assert!(!registry
            .components_by_type_typed::<dyn TestTrait1 + Sync + Send>()
            .is_empty());
        #[cfg(feature = "threadsafe")]
        assert!(!registry
            .components_by_type_typed::<dyn TestTrait2 + Sync + Send>()
            .is_empty());

        #[cfg(not(feature = "threadsafe"))]
        assert!(registry
            .components_by_type_typed::<dyn TestTrait1>()
            .is_some());
        #[cfg(not(feature = "threadsafe"))]
        assert!(registry
            .components_by_type_typed::<dyn TestTrait2>()
            .is_some());
    }

    #[test]
    fn should_register_alias_name() {
        let registry =
            StaticComponentDefinitionRegistry::new(false, &SimpleContextFactory::default())
                .unwrap();
        assert!(registry
            .component_by_name("test_trait_1_alias_name")
            .is_some());
    }
}
