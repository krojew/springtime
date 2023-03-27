use crate::component::{Component, ComponentDowncast, Injectable};
use crate::component_registry::internal::{
    ComponentDefinitionRegisterer, TraitComponentRegisterer,
};
use crate::component_registry::registry::NamedComponentDefinitionMap;
use crate::error::{ComponentDefinitionRegistryError, ComponentInstanceProviderError};
use crate::instance_provider::{ComponentInstanceAnyPtr, ComponentInstanceProvider};
use derivative::Derivative;
use std::any::TypeId;

/// Definition for a [Component] registered in a definition registry.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ComponentDefinition {
    /// Each component has at least one name, which can be used to request a specific instance.
    /// Derive-based components have their name generated from type name by converting it to snake
    /// case.
    pub names: Vec<String>,
    /// With multiple components registered for a given type, one of them can be marked as primary
    /// and returned when requesting a single instance.
    pub is_primary: bool,
    /// Constructor method for type-erased instances.
    #[derivative(Debug = "ignore")]
    pub constructor: fn(
        instance_provider: &dyn ComponentInstanceProvider,
    ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError>,
}

/// Registration information for a [Component]. Please see [ComponentDefinition] for information
/// about the meaning of the fields.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ComponentMetadata {
    pub names: Vec<String>,
    #[derivative(Debug = "ignore")]
    pub constructor: fn(
        instance_provider: &dyn ComponentInstanceProvider,
    ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError>,
}

/// Registration information for an  alias for a [Component] registered in a definition registry.
/// Please see [ComponentDefinition] for information about the meaning of the fields.
#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Debug, Default)]
pub struct ComponentAliasMetadata {
    pub is_primary: bool,
}

/// A registry of component definitions which can be used when requesting instances via a
/// [ComponentInstanceProvider](ComponentInstanceProvider).
pub trait ComponentDefinitionRegistry {
    /// Adds a new definition for a given type. Note: handling of duplicate component names is
    /// registry-dependent.
    fn register_component<T: Component + 'static>(
        &mut self,
        metadata: ComponentMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError>;

    /// Adds an alias `Source` for component of type `Target`. This is useful when registering
    /// `dyn Trait` as an alias for a given concrete type. If `Source` cannot by cast to `Target`,
    /// component creation will fail.
    /// The optional name can be used as an alternative to the name of the concrete component,
    /// therefore making it possible to get the component by multiple names.
    fn register_alias<Source: ComponentDowncast + ?Sized + 'static, Target: Component + 'static>(
        &mut self,
        metadata: ComponentAliasMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError>;

    /// Returns all registered definitions for a given type.
    fn components_by_type<T: Injectable + ?Sized + 'static>(
        &self,
    ) -> Option<Vec<ComponentDefinition>>;

    /// Returns a definition with given name.
    fn component_by_name(&self, name: &str) -> Option<ComponentDefinition>;
}

/// Registry of component definitions initialized from statically registered definitions.
#[derive(Clone, Debug)]
pub struct StaticComponentDefinitionRegistry {
    definitions: NamedComponentDefinitionMap,
    allow_definition_overriding: bool,
}

impl StaticComponentDefinitionRegistry {
    pub fn new(
        allow_definition_overriding: bool,
    ) -> Result<Self, ComponentDefinitionRegistryError> {
        let mut definitions = NamedComponentDefinitionMap::default();
        for registerer in inventory::iter::<ComponentDefinitionRegisterer> {
            let definition = (registerer.register)();
            definitions.try_register_component(
                definition.target,
                definition.metadata,
                allow_definition_overriding,
            )?;
        }

        for registerer in inventory::iter::<TraitComponentRegisterer> {
            let definition = (registerer.register)();
            definitions.try_register_alias(
                definition.trait_type,
                definition.target_type,
                definition.metadata,
            )?;
        }

        Ok(Self {
            definitions,
            allow_definition_overriding,
        })
    }
}

impl ComponentDefinitionRegistry for StaticComponentDefinitionRegistry {
    fn register_component<T: Component + 'static>(
        &mut self,
        metadata: ComponentMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError> {
        self.definitions.try_register_component(
            TypeId::of::<T>(),
            metadata,
            self.allow_definition_overriding,
        )
    }

    #[inline]
    fn register_alias<Source: ComponentDowncast + ?Sized + 'static, Target: Component + 'static>(
        &mut self,
        metadata: ComponentAliasMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError> {
        self.definitions.try_register_alias(
            TypeId::of::<Source>(),
            TypeId::of::<Target>(),
            metadata,
        )
    }

    #[inline]
    fn components_by_type<T: Injectable + ?Sized + 'static>(
        &self,
    ) -> Option<Vec<ComponentDefinition>> {
        self.definitions.components_by_type(TypeId::of::<T>())
    }

    #[inline]
    fn component_by_name(&self, name: &str) -> Option<ComponentDefinition> {
        self.definitions.component_by_name(name)
    }
}

mod registry {
    use crate::component_registry::{
        ComponentAliasMetadata, ComponentDefinition, ComponentMetadata,
    };
    use crate::error::ComponentDefinitionRegistryError;
    use fxhash::FxHashMap;
    use std::any::TypeId;

    #[derive(Default, Clone, Debug)]
    pub(super) struct NamedComponentDefinitionMap {
        definitions: FxHashMap<TypeId, Vec<ComponentDefinition>>,
        names: FxHashMap<String, (TypeId, usize)>,
    }

    impl NamedComponentDefinitionMap {
        pub(super) fn component_by_name(&self, name: &str) -> Option<ComponentDefinition> {
            self.names
                .get(name)
                .and_then(|(id, index)| {
                    self.definitions
                        .get(id)
                        .and_then(|definitions| definitions.get(*index))
                })
                .cloned()
        }

        pub(super) fn components_by_type(
            &self,
            type_id: TypeId,
        ) -> Option<Vec<ComponentDefinition>> {
            self.definitions.get(&type_id).cloned()
        }

        pub(super) fn try_register_alias(
            &mut self,
            alias_type: TypeId,
            target_type: TypeId,
            metadata: ComponentAliasMetadata,
        ) -> Result<(), ComponentDefinitionRegistryError> {
            let mut target_definitions = self
                .definitions
                .get(&target_type)
                .ok_or(ComponentDefinitionRegistryError::MissingBaseComponent {
                    alias_type,
                    target_type,
                })
                .cloned()?;

            for definition in &mut target_definitions {
                definition.is_primary = metadata.is_primary;
            }

            if let Some(alias_definitions) = self.definitions.get_mut(&alias_type) {
                if alias_definitions
                    .iter()
                    .any(|definition| definition.is_primary)
                {
                    return Err(
                        ComponentDefinitionRegistryError::DuplicatePrimaryComponent {
                            alias_type,
                            target_type,
                        },
                    );
                }

                alias_definitions.append(&mut target_definitions);
            } else {
                self.definitions.insert(alias_type, target_definitions);
            }

            Ok(())
        }

        pub(super) fn try_register_component(
            &mut self,
            target: TypeId,
            metadata: ComponentMetadata,
            allow_definition_overriding: bool,
        ) -> Result<(), ComponentDefinitionRegistryError> {
            if !allow_definition_overriding {
                if let Some(name) = metadata.names.iter().find_map(|name| {
                    if self.names.contains_key(name) {
                        Some(name.clone())
                    } else {
                        None
                    }
                }) {
                    return Err(ComponentDefinitionRegistryError::DuplicateComponentName(
                        name,
                    ));
                }
            }

            let definition = ComponentDefinition {
                names: metadata.names,
                is_primary: false,
                constructor: metadata.constructor,
            };

            let names = definition.names.clone();

            if let Some(entries) = self.definitions.get_mut(&target) {
                // concrete component types should not have multiple definitions
                debug_assert!(entries.len() <= 1);

                if !allow_definition_overriding && !entries.is_empty() {
                    return Err(ComponentDefinitionRegistryError::DuplicateComponentType(
                        target,
                    ));
                }

                entries
                    .iter()
                    .flat_map(|entry| entry.names.iter())
                    .for_each(|name| {
                        self.names.remove(name);
                    });

                *entries = vec![definition];
            } else {
                self.definitions.insert(target, vec![definition]);
            }

            self.names
                .extend(names.into_iter().map(|name| (name, (target, 0))));

            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::component_registry::registry::NamedComponentDefinitionMap;
        use crate::component_registry::{ComponentAliasMetadata, ComponentMetadata};
        use crate::error::{ComponentDefinitionRegistryError, ComponentInstanceProviderError};
        use crate::instance_provider::{
            ComponentInstanceAnyPtr, ComponentInstanceProvider, ComponentInstancePtr,
        };
        use std::any::TypeId;

        fn create_metadata() -> (ComponentMetadata, TypeId) {
            fn constructor(
                _instance_provider: &dyn ComponentInstanceProvider,
            ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
                Ok(ComponentInstancePtr::new(0) as ComponentInstanceAnyPtr)
            }

            (
                ComponentMetadata {
                    names: vec!["name".to_string()],
                    constructor,
                },
                TypeId::of::<i8>(),
            )
        }

        #[test]
        fn should_register_definition() {
            let (definition, id) = create_metadata();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, definition.clone(), false)
                .unwrap();

            assert_eq!(
                registry.components_by_type(id).unwrap()[0].names,
                definition.names
            );
            assert_eq!(
                registry.component_by_name("name").unwrap().names,
                definition.names
            );
        }

        #[test]
        fn should_not_register_duplicate_name() {
            let (definition, id) = create_metadata();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, definition.clone(), false)
                .unwrap();

            assert!(matches!(
                registry
                    .try_register_component(id, definition, false)
                    .unwrap_err(),
                ComponentDefinitionRegistryError::DuplicateComponentName(..)
            ));
        }

        #[test]
        fn should_override_duplicate_name() {
            let (definition, id) = create_metadata();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, definition.clone(), true)
                .unwrap();
            registry
                .try_register_component(id, definition, true)
                .unwrap();
        }

        #[test]
        fn should_register_alias() {
            let (definition, id) = create_metadata();
            let alias_id = TypeId::of::<u8>();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, definition.clone(), false)
                .unwrap();
            registry
                .try_register_alias(alias_id, id, ComponentAliasMetadata { is_primary: false })
                .unwrap();

            assert_eq!(
                registry.components_by_type(alias_id).unwrap()[0].names,
                definition.names
            );
        }

        #[test]
        fn should_reject_duplicate_primary_alias() {
            let (definition, id) = create_metadata();
            let alias_id = TypeId::of::<u8>();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, definition, false)
                .unwrap();
            registry
                .try_register_alias(alias_id, id, ComponentAliasMetadata { is_primary: true })
                .unwrap();

            assert!(matches!(
                registry
                    .try_register_alias(alias_id, id, ComponentAliasMetadata { is_primary: true })
                    .unwrap_err(),
                ComponentDefinitionRegistryError::DuplicatePrimaryComponent { .. }
            ));
        }
    }
}

#[doc(hidden)]
pub mod internal {
    use crate::component_registry::{ComponentAliasMetadata, ComponentMetadata};
    use inventory::collect;
    pub use inventory::submit;
    use std::any::TypeId;

    pub struct TypedComponentDefinition {
        pub target: TypeId,
        pub metadata: ComponentMetadata,
    }

    pub struct ComponentDefinitionRegisterer {
        pub register: fn() -> TypedComponentDefinition,
    }

    pub struct TraitComponentDefinition {
        pub trait_type: TypeId,
        pub target_type: TypeId,
        pub metadata: ComponentAliasMetadata,
    }

    pub struct TraitComponentRegisterer {
        pub register: fn() -> TraitComponentDefinition,
    }

    collect!(ComponentDefinitionRegisterer);
    collect!(TraitComponentRegisterer);
}

#[cfg(test)]
mod tests {
    use crate::component::{Component, ComponentDowncast, Injectable};
    use crate::component_registry::{
        ComponentDefinition, ComponentDefinitionRegistry, ComponentMetadata,
        StaticComponentDefinitionRegistry,
    };
    use crate::error::{ComponentDefinitionRegistryError, ComponentInstanceProviderError};
    use crate::instance_provider::{
        ComponentInstanceAnyPtr, ComponentInstanceProvider, ComponentInstancePtr,
    };

    struct TestComponent;

    impl Injectable for TestComponent {}

    impl ComponentDowncast for TestComponent {
        fn downcast(
            source: ComponentInstanceAnyPtr,
        ) -> Result<ComponentInstancePtr<Self>, ComponentInstanceAnyPtr> {
            source.downcast()
        }
    }

    impl Component for TestComponent {
        fn create(
            _instance_provider: &dyn ComponentInstanceProvider,
        ) -> Result<Self, ComponentInstanceProviderError>
        where
            Self: Sized,
        {
            Ok(TestComponent)
        }
    }

    fn test_constructor(
        instance_provider: &dyn ComponentInstanceProvider,
    ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
        TestComponent::create(instance_provider)
            .map(|p| ComponentInstancePtr::new(p) as ComponentInstanceAnyPtr)
    }

    #[test]
    fn should_register_definition() {
        let mut registry = StaticComponentDefinitionRegistry::new(false).unwrap();
        registry
            .register_component::<TestComponent>(ComponentMetadata {
                names: vec!["a".to_string()],
                constructor: test_constructor,
            })
            .unwrap();

        assert!(!registry
            .components_by_type::<TestComponent>()
            .unwrap()
            .is_empty())
    }

    #[test]
    fn should_not_register_duplicate_name() {
        let definition = ComponentDefinition {
            names: vec!["name".to_string()],
            is_primary: false,
            constructor: test_constructor,
        };

        let mut registry = StaticComponentDefinitionRegistry::new(false).unwrap();
        registry
            .register_component::<TestComponent>(ComponentMetadata {
                names: definition.names.clone(),
                constructor: test_constructor,
            })
            .unwrap();

        assert_eq!(
            registry
                .register_component::<TestComponent>(ComponentMetadata {
                    names: definition.names.clone(),
                    constructor: test_constructor,
                })
                .unwrap_err(),
            ComponentDefinitionRegistryError::DuplicateComponentName(definition.names[0].clone())
        );
    }

    #[test]
    fn should_override_duplicate_name() {
        let mut registry = StaticComponentDefinitionRegistry::new(true).unwrap();
        registry
            .register_component::<TestComponent>(ComponentMetadata {
                names: vec!["name".to_string()],
                constructor: test_constructor,
            })
            .unwrap();
        registry
            .register_component::<TestComponent>(ComponentMetadata {
                names: vec!["name2".to_string()],
                constructor: test_constructor,
            })
            .unwrap();

        registry.component_by_name("name2").unwrap();
    }
}
