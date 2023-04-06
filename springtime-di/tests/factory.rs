#[cfg(feature = "derive")]
mod factory_test {
    use springtime_di::factory::ComponentFactoryBuilder;
    use springtime_di::instance_provider::{ComponentInstancePtr, TypedComponentInstanceProvider};
    use springtime_di::{component_alias, injectable, Component};

    #[injectable]
    trait TestTrait1 {}

    #[injectable]
    trait TestTrait2 {}

    #[derive(Component)]
    struct TestDependency1;

    #[derive(Component)]
    struct TestDependency2;

    #[component_alias]
    impl TestTrait1 for TestDependency1 {}

    #[component_alias(primary)]
    impl TestTrait1 for TestDependency2 {}

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
    }

    impl TestComponent {}

    #[test]
    fn should_create_components() {
        let mut component_factory = ComponentFactoryBuilder::new().unwrap().build();

        let component = component_factory.primary_instance_typed::<TestComponent>();
        assert!(component.is_ok());
    }
}
