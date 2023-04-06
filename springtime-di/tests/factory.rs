#[cfg(feature = "derive")]
mod factory_test {
    use springtime_di::component_registry::conditional::unregistered_component;
    use springtime_di::instance_provider::ComponentInstancePtr;
    use springtime_di::{component_alias, injectable, Component};

    #[injectable]
    trait TestTrait1 {}

    #[injectable]
    trait TestTrait2 {}

    #[injectable]
    trait TestTrait3 {}

    #[derive(Component)]
    struct TestDependency1;

    #[derive(Component)]
    struct TestDependency2;

    #[derive(Component)]
    #[cfg_attr(feature = "threadsafe", component(condition = "unregistered_component::<dyn TestTrait3 + Send + Sync>", priority = -100))]
    #[cfg_attr(not(feature = "threadsafe"), component(condition = "unregistered_component::<dyn TestTrait3>", priority = -100))]
    struct TestDependency3;

    #[component_alias]
    impl TestTrait1 for TestDependency1 {}

    #[component_alias]
    impl TestTrait3 for TestDependency1 {}

    #[component_alias(primary)]
    impl TestTrait1 for TestDependency2 {}

    #[component_alias]
    impl TestTrait3 for TestDependency3 {}

    #[derive(Component)]
    struct TestComponent {
        #[cfg(feature = "threadsafe")]
        _dependency_1: ComponentInstancePtr<dyn TestTrait1 + Send + Sync>,
        #[cfg(not(feature = "threadsafe"))]
        _dependency_1: ComponentInstancePtr<dyn TestTrait1>,
        _dependency_2: ComponentInstancePtr<TestDependency1>,
        #[cfg(feature = "threadsafe")]
        _dependency_3: Vec<ComponentInstancePtr<dyn TestTrait1 + Send + Sync>>,
        #[cfg(not(feature = "threadsafe"))]
        _dependency_3: Vec<ComponentInstancePtr<dyn TestTrait1>>,
        #[cfg(feature = "threadsafe")]
        _dependency_4: Option<ComponentInstancePtr<dyn TestTrait2 + Send + Sync>>,
        #[cfg(not(feature = "threadsafe"))]
        _dependency_4: Option<ComponentInstancePtr<dyn TestTrait2>>,
        #[cfg(feature = "threadsafe")]
        #[component(name = "test_dependency_1")]
        _dependency_5: ComponentInstancePtr<dyn TestTrait1 + Send + Sync>,
        #[cfg(not(feature = "threadsafe"))]
        #[component(name = "test_dependency_1")]
        _dependency_5: ComponentInstancePtr<dyn TestTrait1>,
        #[cfg(feature = "threadsafe")]
        _dependency_6: ComponentInstancePtr<dyn TestTrait3 + Send + Sync>,
        #[cfg(not(feature = "threadsafe"))]
        _dependency_6: ComponentInstancePtr<dyn TestTrait3>,
    }

    impl TestComponent {}

    #[cfg(not(feature = "async"))]
    mod sync {
        use crate::factory_test::TestComponent;
        use springtime_di::factory::ComponentFactoryBuilder;
        use springtime_di::instance_provider::TypedComponentInstanceProvider;

        #[test]
        fn should_create_components() {
            let mut component_factory = ComponentFactoryBuilder::new().unwrap().build();

            let component = component_factory.primary_instance_typed::<TestComponent>();
            assert!(component.is_ok());
        }
    }
}
