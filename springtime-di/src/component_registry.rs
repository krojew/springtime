use crate::component::{Component, ComponentDowncast, Injectable};
use crate::component_registry::internal::{
    ComponentDefinitionRegisterer, TraitComponentRegisterer,
};
use crate::component_registry::registry::NamedComponentDefinitionMap;
use crate::error::{ComponentDefinitionRegistryError, ComponentInstanceProviderError};
use crate::instance_provider::{CastFunction, ComponentInstanceAnyPtr, ComponentInstanceProvider};
use derivative::Derivative;
use std::any::{type_name, TypeId};

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
        instance_provider: &mut dyn ComponentInstanceProvider,
    ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError>,

    /// Cast function associated for given type. Please see the documentation for [CastFunction] for
    /// details on usage.
    #[derivative(Debug = "ignore")]
    pub cast: CastFunction,
}

/// Registration information for a [Component]. Please see [ComponentDefinition] for information
/// about the meaning of the fields.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ComponentMetadata {
    pub names: Vec<String>,

    #[derivative(Debug = "ignore")]
    pub constructor: fn(
        instance_provider: &mut dyn ComponentInstanceProvider,
    ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError>,

    #[derivative(Debug = "ignore")]
    pub cast: CastFunction,
}

/// Registration information for an  alias for a [Component] registered in a definition registry.
/// Please see [ComponentDefinition] for information about the meaning of the fields.
#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct ComponentAliasMetadata {
    pub is_primary: bool,

    #[derivative(Debug = "ignore")]
    pub cast: CastFunction,
}

/// A registry of component definitions which can be used when requesting instances via a
/// [ComponentInstanceProvider](ComponentInstanceProvider).
pub trait ComponentDefinitionRegistry {
    /// Adds a new definition for a given type. Note: handling of duplicate component names is
    /// registry-dependent.
    fn register_component<T: Component>(
        &mut self,
        metadata: ComponentMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError>;

    /// Adds an alias `Source` for component of type `Target`. This is useful when registering
    /// `dyn Trait` as an alias for a given concrete type. If `Source` cannot by cast to `Target`,
    /// component creation will fail.
    /// The optional name can be used as an alternative to the name of the concrete component,
    /// therefore making it possible to get the component by multiple names.
    fn register_alias<Source: ComponentDowncast<Target> + ?Sized, Target: Component>(
        &mut self,
        metadata: ComponentAliasMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError>;

    /// Returns all registered definitions for a given type.
    fn components_by_type<T: Injectable + ?Sized>(&self) -> Option<Vec<ComponentDefinition>>;

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
                definition.target_name,
                definition.metadata,
                allow_definition_overriding,
            )?;
        }

        for registerer in inventory::iter::<TraitComponentRegisterer> {
            let definition = (registerer.register)();
            definitions.try_register_alias(
                definition.trait_type,
                definition.target_type,
                definition.trait_name,
                definition.target_name,
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
    fn register_component<T: Component>(
        &mut self,
        metadata: ComponentMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError> {
        self.definitions.try_register_component(
            TypeId::of::<T>(),
            type_name::<T>(),
            metadata,
            self.allow_definition_overriding,
        )
    }

    #[inline]
    fn register_alias<Source: ComponentDowncast<Target> + ?Sized, Target: Component>(
        &mut self,
        metadata: ComponentAliasMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError> {
        self.definitions.try_register_alias(
            TypeId::of::<Source>(),
            TypeId::of::<Target>(),
            type_name::<Source>(),
            type_name::<Target>(),
            metadata,
        )
    }

    #[inline]
    fn components_by_type<T: Injectable + ?Sized>(&self) -> Option<Vec<ComponentDefinition>> {
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
            alias_name: &str,
            target_name: &str,
            metadata: ComponentAliasMetadata,
        ) -> Result<(), ComponentDefinitionRegistryError> {
            let mut target_definitions = self
                .definitions
                .get(&target_type)
                .ok_or(ComponentDefinitionRegistryError::MissingBaseComponent {
                    alias_type: alias_name.to_string(),
                    target_type: target_name.to_string(),
                })
                .cloned()?;

            for definition in &mut target_definitions {
                definition.is_primary = metadata.is_primary;
            }

            if let Some(alias_definitions) = self.definitions.get_mut(&alias_type) {
                if metadata.is_primary
                    && alias_definitions
                        .iter()
                        .any(|definition| definition.is_primary)
                {
                    return Err(
                        ComponentDefinitionRegistryError::DuplicatePrimaryComponent {
                            alias_type: alias_name.to_string(),
                            target_type: target_name.to_string(),
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
            target_name: &str,
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
                cast: metadata.cast,
            };

            let names = definition.names.clone();

            if let Some(entries) = self.definitions.get_mut(&target) {
                // concrete component types should not have multiple definitions
                debug_assert!(entries.len() <= 1);

                if !allow_definition_overriding && !entries.is_empty() {
                    return Err(ComponentDefinitionRegistryError::DuplicateComponentType(
                        target_name.to_string(),
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

        fn cast(
            instance: ComponentInstanceAnyPtr,
            _result: *mut (),
        ) -> Result<(), ComponentInstanceAnyPtr> {
            Err(instance)
        }

        fn create_metadata() -> (ComponentMetadata, TypeId) {
            fn constructor(
                _instance_provider: &mut dyn ComponentInstanceProvider,
            ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
                Ok(ComponentInstancePtr::new(0) as ComponentInstanceAnyPtr)
            }

            (
                ComponentMetadata {
                    names: vec!["name".to_string()],
                    constructor,
                    cast,
                },
                TypeId::of::<i8>(),
            )
        }

        #[test]
        fn should_register_definition() {
            let (definition, id) = create_metadata();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, "", definition.clone(), false)
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
                .try_register_component(id, "", definition.clone(), false)
                .unwrap();

            assert!(matches!(
                registry
                    .try_register_component(id, "", definition, false)
                    .unwrap_err(),
                ComponentDefinitionRegistryError::DuplicateComponentName(..)
            ));
        }

        #[test]
        fn should_override_duplicate_name() {
            let (definition, id) = create_metadata();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, "", definition.clone(), true)
                .unwrap();
            registry
                .try_register_component(id, "", definition, true)
                .unwrap();
        }

        #[test]
        fn should_register_alias() {
            let (definition, id) = create_metadata();
            let alias_id = TypeId::of::<u8>();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, "", definition.clone(), false)
                .unwrap();
            registry
                .try_register_alias(
                    alias_id,
                    id,
                    "",
                    "",
                    ComponentAliasMetadata {
                        is_primary: false,
                        cast,
                    },
                )
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
                .try_register_component(id, "", definition, false)
                .unwrap();
            registry
                .try_register_alias(
                    alias_id,
                    id,
                    "",
                    "",
                    ComponentAliasMetadata {
                        is_primary: true,
                        cast,
                    },
                )
                .unwrap();

            assert!(matches!(
                registry
                    .try_register_alias(
                        alias_id,
                        id,
                        "",
                        "",
                        ComponentAliasMetadata {
                            is_primary: true,
                            cast
                        }
                    )
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
        pub target_name: &'static str,
        pub metadata: ComponentMetadata,
    }

    pub struct ComponentDefinitionRegisterer {
        pub register: fn() -> TypedComponentDefinition,
    }

    pub struct TraitComponentDefinition {
        pub trait_type: TypeId,
        pub target_type: TypeId,
        pub trait_name: &'static str,
        pub target_name: &'static str,
        pub metadata: ComponentAliasMetadata,
    }

    pub struct TraitComponentRegisterer {
        pub register: fn() -> TraitComponentDefinition,
    }

    collect!(ComponentDefinitionRegisterer);
    collect!(TraitComponentRegisterer);
}

#[cfg(test)]
//noinspection DuplicatedCode
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

    impl ComponentDowncast<TestComponent> for TestComponent {
        fn downcast(
            source: ComponentInstanceAnyPtr,
        ) -> Result<ComponentInstancePtr<Self>, ComponentInstanceAnyPtr> {
            source.downcast()
        }
    }

    impl Component for TestComponent {
        fn create(
            _instance_provider: &mut dyn ComponentInstanceProvider,
        ) -> Result<Self, ComponentInstanceProviderError>
        where
            Self: Sized,
        {
            Ok(TestComponent)
        }
    }

    fn test_constructor(
        instance_provider: &mut dyn ComponentInstanceProvider,
    ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
        TestComponent::create(instance_provider)
            .map(|p| ComponentInstancePtr::new(p) as ComponentInstanceAnyPtr)
    }

    unsafe fn test_cast(
        instance: ComponentInstanceAnyPtr,
        result: *mut (),
    ) -> Result<(), ComponentInstanceAnyPtr> {
        let p = TestComponent::downcast(instance)?;
        let result = &mut *(result as *mut Option<ComponentInstancePtr<TestComponent>>);
        *result = Some(p);
        Ok(())
    }

    #[test]
    fn should_register_definition() {
        let mut registry = StaticComponentDefinitionRegistry::new(false).unwrap();
        registry
            .register_component::<TestComponent>(ComponentMetadata {
                names: vec!["a".to_string()],
                constructor: test_constructor,
                cast: test_cast,
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
            cast: test_cast,
        };

        let mut registry = StaticComponentDefinitionRegistry::new(false).unwrap();
        registry
            .register_component::<TestComponent>(ComponentMetadata {
                names: definition.names.clone(),
                constructor: test_constructor,
                cast: test_cast,
            })
            .unwrap();

        assert_eq!(
            registry
                .register_component::<TestComponent>(ComponentMetadata {
                    names: definition.names.clone(),
                    constructor: test_constructor,
                    cast: test_cast,
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
                cast: test_cast,
            })
            .unwrap();
        registry
            .register_component::<TestComponent>(ComponentMetadata {
                names: vec!["name2".to_string()],
                constructor: test_constructor,
                cast: test_cast,
            })
            .unwrap();

        registry.component_by_name("name2").unwrap();
    }
}
