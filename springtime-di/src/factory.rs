//! Core functionality for creating [Component](crate::component::Component) instances.

use crate::component_registry::conditional::SimpleContextFactory;
use crate::component_registry::{
    ComponentDefinition, ComponentDefinitionRegistry, ComponentDefinitionRegistryError,
    StaticComponentDefinitionRegistry,
};
use crate::instance_provider::{
    CastFunction, ComponentInstanceAnyPtr, ComponentInstanceProvider,
    ComponentInstanceProviderError,
};
use crate::scope::{
    PrototypeScopeFactory, ScopeFactory, ScopePtr, SingletonScopeFactory, PROTOTYPE, SINGLETON,
};
#[cfg(feature = "async")]
use futures::future::BoxFuture;
#[cfg(feature = "async")]
use futures::FutureExt;
use fxhash::{FxHashMap, FxHashSet};
#[cfg(not(feature = "async"))]
use itertools::Itertools;
use std::any::TypeId;
use tracing::debug;

#[cfg(not(feature = "threadsafe"))]
pub type ComponentDefinitionRegistryPtr = Box<dyn ComponentDefinitionRegistry>;
#[cfg(feature = "threadsafe")]
pub type ComponentDefinitionRegistryPtr = Box<dyn ComponentDefinitionRegistry + Send + Sync>;

#[cfg(not(feature = "threadsafe"))]
pub type ScopeFactoryPtr = Box<dyn ScopeFactory>;
#[cfg(feature = "threadsafe")]
pub type ScopeFactoryPtr = Box<dyn ScopeFactory + Send + Sync>;

pub type ScopeFactoryRegistry = FxHashMap<String, ScopeFactoryPtr>;

/// Builder for [ComponentFactory] with sensible defaults, for easy construction.
pub struct ComponentFactoryBuilder {
    definition_registry: ComponentDefinitionRegistryPtr,
    scope_factories: ScopeFactoryRegistry,
}

impl ComponentFactoryBuilder {
    /// Creates a new builder with a default configuration.
    pub fn new() -> Result<Self, ComponentDefinitionRegistryError> {
        Ok(Self {
            definition_registry: Box::new(StaticComponentDefinitionRegistry::new(
                true,
                &SimpleContextFactory,
            )?),
            scope_factories: [
                (
                    SINGLETON.to_string(),
                    Box::<SingletonScopeFactory>::default() as ScopeFactoryPtr,
                ),
                (
                    PROTOTYPE.to_string(),
                    Box::<PrototypeScopeFactory>::default() as ScopeFactoryPtr,
                ),
            ]
            .into_iter()
            .collect(),
        })
    }

    /// Sets new [ComponentDefinitionRegistry].
    pub fn with_definition_registry(
        mut self,
        definition_registry: ComponentDefinitionRegistryPtr,
    ) -> Self {
        self.definition_registry = definition_registry;
        self
    }

    /// Sets new scope factories.
    pub fn with_scope_factories(mut self, scope_factories: ScopeFactoryRegistry) -> Self {
        self.scope_factories = scope_factories;
        self
    }

    /// Adds a new scope factory.
    pub fn with_scope_factory<T: ToString>(mut self, name: T, factory: ScopeFactoryPtr) -> Self {
        self.scope_factories.insert(name.to_string(), factory);
        self
    }

    /// Builds resulting [ComponentFactory].
    pub fn build(self) -> ComponentFactory {
        ComponentFactory::new(self.definition_registry, self.scope_factories)
    }
}

/// Generic factory for [Component](crate::component::Component) instances. Uses definitions from
/// the [ComponentDefinitionRegistry] and [scopes](crate::scope) to create and store instances for
/// reuse.
pub struct ComponentFactory {
    definition_registry: ComponentDefinitionRegistryPtr,
    scope_factories: FxHashMap<String, ScopeFactoryPtr>,
    scopes: FxHashMap<String, ScopePtr>,
    types_under_construction: FxHashSet<TypeId>,
}

impl ComponentFactory {
    /// Creates a new factory with given registry and scope factories. The factory map should
    /// include built-in [SINGLETON] and [PROTOTYPE] for maximum compatibility with components,
    /// since they are usually the most popular. This is not a hard requirement, but care needs to
    /// be taken to ensue no component uses them.
    pub fn new(
        definition_registry: ComponentDefinitionRegistryPtr,
        scope_factories: FxHashMap<String, ScopeFactoryPtr>,
    ) -> Self {
        Self {
            definition_registry,
            scope_factories,
            scopes: Default::default(),
            types_under_construction: Default::default(),
        }
    }

    #[cfg(feature = "async")]
    async fn call_constructor(
        &mut self,
        definition: &ComponentDefinition,
    ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
        self.types_under_construction
            .insert(definition.resolved_type_id);
        let instance = (definition.constructor)(self).await;
        self.types_under_construction
            .remove(&definition.resolved_type_id);

        instance
    }

    #[cfg(not(feature = "async"))]
    fn call_constructor(
        &mut self,
        definition: &ComponentDefinition,
    ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
        self.types_under_construction
            .insert(definition.resolved_type_id);
        let instance = (definition.constructor)(self);
        self.types_under_construction
            .remove(&definition.resolved_type_id);

        instance
    }

    fn check_scope_instance(
        &mut self,
        definition: &ComponentDefinition,
    ) -> Result<Option<(ComponentInstanceAnyPtr, CastFunction)>, ComponentInstanceProviderError>
    {
        if self
            .types_under_construction
            .contains(&definition.resolved_type_id)
        {
            return Err(ComponentInstanceProviderError::DependencyCycle {
                type_id: definition.resolved_type_id,
                type_name: None,
            });
        }

        let scope = {
            if let Some(scope) = self.scopes.get(&definition.scope) {
                scope
            } else {
                let factory = self.scope_factories.get(&definition.scope).ok_or_else(|| {
                    ComponentInstanceProviderError::UnrecognizedScope(definition.scope.to_string())
                })?;

                self.scopes
                    .entry(definition.scope.clone())
                    .or_insert(factory.create_scope())
            }
        };

        Ok(scope
            .instance(definition)
            .map(|instance| (instance, definition.cast)))
    }

    fn store_instance_in_scope(
        &mut self,
        definition: &ComponentDefinition,
        instance: ComponentInstanceAnyPtr,
    ) -> Result<(), ComponentInstanceProviderError> {
        let scope = self.scopes.get_mut(&definition.scope).ok_or_else(|| {
            ComponentInstanceProviderError::UnrecognizedScope(definition.scope.to_string())
        })?;

        scope.store_instance(definition, instance);

        Ok(())
    }

    #[cfg(feature = "async")]
    async fn create_instance(
        &mut self,
        definition: &ComponentDefinition,
    ) -> Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError> {
        if let Some(instance) = self.check_scope_instance(definition)? {
            return Ok(instance);
        }

        debug!(
            resolved_type_name = definition.resolved_type_name,
            "Creating new component instance."
        );

        let instance = self.call_constructor(definition).await?;

        self.store_instance_in_scope(definition, instance.clone())?;
        Ok((instance, definition.cast))
    }

    #[cfg(not(feature = "async"))]
    fn create_instance(
        &mut self,
        definition: &ComponentDefinition,
    ) -> Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError> {
        if let Some(instance) = self.check_scope_instance(definition)? {
            return Ok(instance);
        }

        debug!(
            resolved_type_name = definition.resolved_type_name,
            "Creating new component instance."
        );

        let instance = self.call_constructor(definition)?;

        self.store_instance_in_scope(definition, instance.clone())?;
        Ok((instance, definition.cast))
    }
}

impl ComponentInstanceProvider for ComponentFactory {
    #[cfg(feature = "async")]
    fn primary_instance(
        &mut self,
        type_id: TypeId,
    ) -> BoxFuture<
        '_,
        Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError>,
    > {
        async move {
            let definition = self.definition_registry.primary_component(type_id).ok_or(
                ComponentInstanceProviderError::NoPrimaryInstance {
                    type_id,
                    type_name: None,
                },
            )?;

            self.create_instance(&definition).await
        }
        .boxed()
    }

    #[cfg(not(feature = "async"))]
    fn primary_instance(
        &mut self,
        type_id: TypeId,
    ) -> Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError> {
        let definition = self.definition_registry.primary_component(type_id).ok_or(
            ComponentInstanceProviderError::NoPrimaryInstance {
                type_id,
                type_name: None,
            },
        )?;

        self.create_instance(&definition)
    }

    #[cfg(feature = "async")]
    fn instances(
        &mut self,
        type_id: TypeId,
    ) -> BoxFuture<
        '_,
        Result<Vec<(ComponentInstanceAnyPtr, CastFunction)>, ComponentInstanceProviderError>,
    > {
        async move {
            let definitions = self.definition_registry.components_by_type(type_id);

            let mut result = Vec::with_capacity(definitions.len());
            for definition in &definitions {
                result.push(self.create_instance(definition).await?);
            }

            Ok(result)
        }
        .boxed()
    }

    #[cfg(not(feature = "async"))]
    fn instances(
        &mut self,
        type_id: TypeId,
    ) -> Result<Vec<(ComponentInstanceAnyPtr, CastFunction)>, ComponentInstanceProviderError> {
        self.definition_registry
            .components_by_type(type_id)
            .iter()
            .map(|definition| self.create_instance(definition))
            .try_collect()
    }

    #[cfg(feature = "async")]
    fn instance_by_name(
        &mut self,
        name: &str,
        type_id: TypeId,
    ) -> BoxFuture<
        '_,
        Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError>,
    > {
        let name = name.to_string();
        async move {
            let definition = self
                .definition_registry
                .component_by_name(&name, type_id)
                .ok_or_else(|| ComponentInstanceProviderError::NoNamedInstance(name.to_string()))?;

            self.create_instance(&definition).await
        }
        .boxed()
    }

    #[cfg(not(feature = "async"))]
    fn instance_by_name(
        &mut self,
        name: &str,
        type_id: TypeId,
    ) -> Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError> {
        let definition = self
            .definition_registry
            .component_by_name(name, type_id)
            .ok_or_else(|| ComponentInstanceProviderError::NoNamedInstance(name.to_string()))?;

        self.create_instance(&definition)
    }
}

//noinspection DuplicatedCode
#[cfg(test)]
mod tests {
    #[cfg(not(feature = "async"))]
    mod sync {
        use crate::component_registry::{
            ComponentDefinition, ComponentDefinitionRegistry, MockComponentDefinitionRegistry,
        };
        use crate::factory::{ComponentDefinitionRegistryPtr, ComponentFactory, ScopeFactoryPtr};
        use crate::instance_provider::{
            ComponentInstanceAnyPtr, ComponentInstanceProvider, ComponentInstanceProviderError,
            ComponentInstancePtr,
        };
        use crate::scope::{
            MockScope, MockScopeFactory, PrototypeScopeFactory, ScopePtr, PROTOTYPE, SINGLETON,
        };
        use mockall::predicate::*;
        use std::any::{type_name, Any, TypeId};

        fn cast(
            instance: ComponentInstanceAnyPtr,
        ) -> Result<Box<dyn Any>, ComponentInstanceAnyPtr> {
            Err(instance)
        }

        fn constructor(
            _instance_provider: &mut dyn ComponentInstanceProvider,
        ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
            Ok(ComponentInstancePtr::new(0) as ComponentInstanceAnyPtr)
        }

        fn error_constructor(
            _instance_provider: &mut dyn ComponentInstanceProvider,
        ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
            Err(ComponentInstanceProviderError::NoPrimaryInstance {
                type_id: TypeId::of::<i8>(),
                type_name: None,
            })
        }

        fn recursive_constructor(
            instance_provider: &mut dyn ComponentInstanceProvider,
        ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
            instance_provider
                .primary_instance(TypeId::of::<i8>())
                .map(|(instance, _)| instance)
        }

        fn create_definition() -> (ComponentDefinition, TypeId) {
            (
                ComponentDefinition {
                    names: ["name".to_string()].into_iter().collect(),
                    is_primary: false,
                    scope: PROTOTYPE.to_string(),
                    resolved_type_id: TypeId::of::<i8>(),
                    resolved_type_name: type_name::<i8>().to_string(),
                    constructor,
                    cast,
                },
                TypeId::of::<i8>(),
            )
        }

        fn create_factory<T: ComponentDefinitionRegistry + Send + Sync + 'static>(
            definition_registry: T,
        ) -> ComponentFactory {
            ComponentFactory::new(
                Box::new(definition_registry) as ComponentDefinitionRegistryPtr,
                [(
                    PROTOTYPE.to_string(),
                    Box::<PrototypeScopeFactory>::default() as ScopeFactoryPtr,
                )]
                .into_iter()
                .collect(),
            )
        }

        #[test]
        fn should_return_primary_instance() {
            let (definition, id) = create_definition();

            let mut registry = MockComponentDefinitionRegistry::new();
            registry
                .expect_primary_component()
                .with(eq(id))
                .times(1)
                .return_const(Some(definition));

            let mut factory = create_factory(registry);
            assert!(factory.primary_instance(id).is_ok());
        }

        #[test]
        fn should_detect_primary_instance_loops() {
            let id = TypeId::of::<i8>();
            let definition = ComponentDefinition {
                names: Default::default(),
                is_primary: false,
                scope: PROTOTYPE.to_string(),
                resolved_type_id: TypeId::of::<i8>(),
                resolved_type_name: type_name::<i8>().to_string(),
                constructor: recursive_constructor,
                cast,
            };

            let mut registry = MockComponentDefinitionRegistry::new();
            registry
                .expect_primary_component()
                .with(eq(id))
                .times(2)
                .return_const(Some(definition));

            let mut factory = create_factory(registry);
            assert!(matches!(
                factory.primary_instance(id).unwrap_err(),
                ComponentInstanceProviderError::DependencyCycle { type_id, .. } if type_id == TypeId::of::<i8>()
            ));
        }

        #[test]
        fn should_not_return_missing_primary_instance() {
            let id = TypeId::of::<i8>();

            let mut registry = MockComponentDefinitionRegistry::new();
            registry
                .expect_primary_component()
                .with(eq(id))
                .times(1)
                .return_const(None);

            let mut factory = create_factory(registry);
            assert!(matches!(
                factory.primary_instance(id).unwrap_err(),
                ComponentInstanceProviderError::NoPrimaryInstance { type_id, .. } if type_id == TypeId::of::<i8>()
            ));
        }

        #[test]
        fn should_recognize_primary_instance_missing_scope() {
            let id = TypeId::of::<i8>();
            let definition = ComponentDefinition {
                names: Default::default(),
                is_primary: false,
                scope: SINGLETON.to_string(),
                resolved_type_id: TypeId::of::<i8>(),
                resolved_type_name: type_name::<i8>().to_string(),
                constructor,
                cast,
            };

            let mut registry = MockComponentDefinitionRegistry::new();
            registry
                .expect_primary_component()
                .with(eq(id))
                .times(1)
                .return_const(Some(definition));

            let mut factory = create_factory(registry);
            assert!(matches!(
                factory.primary_instance(id).unwrap_err(),
                ComponentInstanceProviderError::UnrecognizedScope(scope) if scope == SINGLETON
            ));
        }

        #[test]
        fn should_forward_primary_instance_constructor_error() {
            let id = TypeId::of::<i8>();
            let definition = ComponentDefinition {
                names: Default::default(),
                is_primary: false,
                scope: PROTOTYPE.to_string(),
                resolved_type_id: TypeId::of::<i8>(),
                resolved_type_name: type_name::<i8>().to_string(),
                constructor: error_constructor,
                cast,
            };

            let mut registry = MockComponentDefinitionRegistry::new();
            registry
                .expect_primary_component()
                .with(eq(id))
                .times(1)
                .return_const(Some(definition));

            let mut factory = create_factory(registry);
            assert!(matches!(
                factory.primary_instance(id).unwrap_err(),
                ComponentInstanceProviderError::NoPrimaryInstance { .. }
            ));
        }

        #[test]
        fn should_store_primary_instance_in_scope() {
            let (definition, id) = create_definition();

            let mut registry = MockComponentDefinitionRegistry::new();
            registry
                .expect_primary_component()
                .with(eq(id))
                .times(1)
                .return_const(Some(definition));

            let mut scope_factory = MockScopeFactory::new();
            scope_factory.expect_create_scope().returning(|| {
                let mut scope = MockScope::new();
                scope.expect_store_instance().times(1).return_const(());
                scope.expect_instance().return_const(None);

                Box::new(scope) as ScopePtr
            });

            let mut factory = ComponentFactory::new(
                Box::new(registry) as ComponentDefinitionRegistryPtr,
                [(
                    PROTOTYPE.to_string(),
                    Box::new(scope_factory) as ScopeFactoryPtr,
                )]
                .into_iter()
                .collect(),
            );

            factory.primary_instance(id).unwrap();
        }

        #[test]
        fn should_return_all_instances() {
            let (definition, id) = create_definition();

            let mut registry = MockComponentDefinitionRegistry::new();
            registry
                .expect_components_by_type()
                .with(eq(id))
                .times(1)
                .return_const(vec![definition.clone(), definition]);

            let mut factory = create_factory(registry);
            assert_eq!(factory.instances(id).unwrap().len(), 2);
        }

        #[test]
        fn should_return_instance_by_name() {
            let (definition, id) = create_definition();

            let mut registry = MockComponentDefinitionRegistry::new();
            registry
                .expect_component_by_name()
                .with(eq("name"), eq(id))
                .times(1)
                .return_const(Some(definition));

            let mut factory = create_factory(registry);
            assert!(factory.instance_by_name("name", id).is_ok());
        }
    }
}
