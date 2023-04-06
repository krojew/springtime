//! Functionality related to registering definitions of components. [ComponentInstanceProvider]s
//! should create [Component] instances based on those definitions, which can be registered
//! automatically or manually.

pub mod conditional;

use crate::component::{Component, ComponentDowncast, Injectable};
use crate::component_registry::conditional::{
    ComponentDefinitionRegistryFacade, ConditionMetadata, ContextFactory,
};
use crate::component_registry::internal::{
    ComponentAliasDefinition, ComponentAliasRegisterer, ComponentDefinitionRegisterer,
    TypedComponentDefinition,
};
use crate::component_registry::registry::NamedComponentDefinitionMap;
use crate::instance_provider::{
    CastFunction, ComponentInstanceAnyPtr, ComponentInstanceProvider,
    ComponentInstanceProviderError,
};
use derivative::Derivative;
use fxhash::{FxHashMap, FxHashSet};
use itertools::Itertools;
#[cfg(test)]
use mockall::automock;
use std::any::{type_name, TypeId};
use thiserror::Error;

/// Error related to component registries.
#[derive(Error, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum ComponentDefinitionRegistryError {
    #[error("Attempted to register a duplicated component with name: {0}")]
    DuplicateComponentName(String),
    #[error("Attempted to re-register a concrete component type: {0}")]
    DuplicateComponentType(String),
    #[error("Missing base component of type {target_type} for alias: {alias_type}")]
    MissingBaseComponent {
        alias_type: String,
        target_type: String,
    },
    #[error(
        "Registering a duplicate primary component of type {target_type} for alias: {alias_type}"
    )]
    DuplicatePrimaryComponent {
        alias_type: String,
        target_type: String,
    },
    #[error("Cannot register alias {alias_type} named {name} for multiple targets: {target_type}")]
    CannotRegisterNamedAliasForAmbiguousTarget {
        alias_type: String,
        target_type: String,
        name: String,
    },
}

/// Definition for a [Component] registered in a definition registry.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct ComponentDefinition {
    /// Each component has at least one name, which can be used to request a specific instance.
    /// Derive-based components have their name generated from type name by converting it to snake
    /// case.
    pub names: FxHashSet<String>,

    /// With multiple components registered for a given type, one of them can be marked as primary
    /// and returned when requesting a single instance.
    pub is_primary: bool,

    /// Which type of [Scope](crate::scope::Scope) to use when requesting given component. Please
    /// see [scope](crate::scope) for details on scopes.
    pub scope_name: String,

    /// Concrete component type id. Since aliases can share definitions with their targets, there
    /// can be a need to find out what is the leaf type.
    pub resolved_type_id: TypeId,

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
    pub names: FxHashSet<String>,

    pub scope_name: String,

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

    pub name: Option<String>,

    pub scope_name: String,

    #[derivative(Debug = "ignore")]
    pub cast: CastFunction,
}

/// A registry of component definitions which can be used when requesting instances via a
/// [ComponentInstanceProvider].
#[cfg_attr(test, automock)]
pub trait ComponentDefinitionRegistry {
    /// Adds a new definition for a given type. Note: handling of duplicate component names is
    /// registry-dependent. Name is used for reporting purposes.
    fn register_component(
        &mut self,
        target: TypeId,
        target_name: &str,
        metadata: &ComponentMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError>;

    /// Adds an alias for a component of target type. This is useful when registering
    /// `dyn Trait` as an alias for a given concrete type. If alias cannot by cast to target,
    /// component creation will fail. Names are used for reporting purposes.
    fn register_alias(
        &mut self,
        alias_type: TypeId,
        target_type: TypeId,
        alias_name: &str,
        target_name: &str,
        metadata: &ComponentAliasMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError>;

    /// Returns all registered definitions for a given type.
    fn components_by_type(&self, type_id: TypeId) -> Vec<ComponentDefinition>;

    /// Returns a definition with given name.
    fn component_by_name(&self, name: &str) -> Option<ComponentDefinition>;

    /// Returns primary component for a given type.
    fn primary_component(&self, type_id: TypeId) -> Option<ComponentDefinition>;

    /// Checks if given type is present in this registry.
    fn is_registered(&self, type_id: TypeId) -> bool;

    /// Checks if there's a definition with given name.
    fn is_name_registered(&self, name: &str) -> bool;

    /// Returns a copy of the whole registry as a map.
    fn all_definitions(&self) -> FxHashMap<TypeId, Vec<ComponentDefinition>>;
}

/// Helper trait for [ComponentDefinitionRegistry] providing strongly-typed access.
pub trait TypedComponentDefinitionRegistry {
    /// Typesafe version of [ComponentDefinitionRegistry::register_component].
    fn register_component_typed<T: Component>(
        &mut self,
        metadata: &ComponentMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError>;

    /// Typesafe version of [ComponentDefinitionRegistry::register_alias].
    fn register_alias_typed<Source: ComponentDowncast<Target> + ?Sized, Target: Component>(
        &mut self,
        metadata: &ComponentAliasMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError>;

    /// Typesafe version of [ComponentDefinitionRegistry::components_by_type].
    fn components_by_type_typed<T: Injectable + ?Sized>(&self) -> Vec<ComponentDefinition>;

    /// Typesafe version of [ComponentDefinitionRegistry::primary_component].
    fn primary_component_typed<T: Injectable + ?Sized>(&self) -> Option<ComponentDefinition>;

    /// Typesafe version of [ComponentDefinitionRegistry::is_registered].
    fn is_registered_typed<T: Injectable>(&self) -> bool;
}

impl<CDR: ComponentDefinitionRegistry + ?Sized> TypedComponentDefinitionRegistry for CDR {
    #[inline]
    fn register_component_typed<T: Component>(
        &mut self,
        metadata: &ComponentMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError> {
        self.register_component(TypeId::of::<T>(), type_name::<T>(), metadata)
    }

    #[inline]
    fn register_alias_typed<Source: ComponentDowncast<Target> + ?Sized, Target: Component>(
        &mut self,
        metadata: &ComponentAliasMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError> {
        self.register_alias(
            TypeId::of::<Source>(),
            TypeId::of::<Target>(),
            type_name::<Source>(),
            type_name::<Target>(),
            metadata,
        )
    }

    #[inline]
    fn components_by_type_typed<T: Injectable + ?Sized>(&self) -> Vec<ComponentDefinition> {
        self.components_by_type(TypeId::of::<T>())
    }

    #[inline]
    fn primary_component_typed<T: Injectable + ?Sized>(&self) -> Option<ComponentDefinition> {
        self.primary_component(TypeId::of::<T>())
    }

    #[inline]
    fn is_registered_typed<T: Injectable>(&self) -> bool {
        self.is_registered(TypeId::of::<T>())
    }
}

/// Registry of component definitions initialized from statically registered definitions.
#[derive(Clone, Debug)]
pub struct StaticComponentDefinitionRegistry {
    definition_map: NamedComponentDefinitionMap,
    allow_definition_overriding: bool,
}

impl StaticComponentDefinitionRegistry {
    pub fn new<CF: ContextFactory>(
        allow_definition_overriding: bool,
        context_factory: &CF,
    ) -> Result<Self, ComponentDefinitionRegistryError> {
        let component_definitions: Vec<TypedComponentDefinition> =
            inventory::iter::<ComponentDefinitionRegisterer>
                .into_iter()
                .map(|registerer| (registerer.register)())
                .collect_vec();

        let alias_definitions: Vec<ComponentAliasDefinition> =
            inventory::iter::<ComponentAliasRegisterer>
                .into_iter()
                .map(|registerer| (registerer.register)())
                .collect_vec();

        let definition_map = Self::register_unconditional_components(
            &component_definitions,
            allow_definition_overriding,
        )?;

        let mut registry = Self {
            definition_map,
            allow_definition_overriding,
        };

        registry.register_conditional_components(
            component_definitions,
            alias_definitions.clone(),
            context_factory,
        )?;

        registry.register_unconditional_aliases(&alias_definitions)?;

        Ok(registry)
    }

    //noinspection DuplicatedCode
    fn register_conditional_components<CF: ContextFactory>(
        &mut self,
        component_definitions: Vec<TypedComponentDefinition>,
        alias_definitions: Vec<ComponentAliasDefinition>,
        context_factory: &CF,
    ) -> Result<(), ComponentDefinitionRegistryError> {
        if component_definitions.is_empty() && alias_definitions.is_empty() {
            return Ok(());
        }

        for (definition, condition) in component_definitions
            .iter()
            .filter_map(|definition| {
                definition
                    .condition
                    .map(|condition| (definition, condition))
            })
            .sorted_by_key(|(definition, _)| -definition.priority)
        {
            if (condition)(
                context_factory.create_context(self).as_ref(),
                ConditionMetadata::Component(&definition.metadata),
            ) {
                self.definition_map.try_register_component(
                    definition.target,
                    definition.target_name,
                    &definition.metadata,
                    self.allow_definition_overriding,
                )?;
            }
        }

        for (definition, condition) in alias_definitions
            .iter()
            .filter_map(|definition| {
                definition
                    .condition
                    .map(|condition| (definition, condition))
            })
            .sorted_by_key(|(definition, _)| -definition.priority)
        {
            if (condition)(
                context_factory.create_context(self).as_ref(),
                ConditionMetadata::Alias(&definition.metadata),
            ) {
                self.definition_map.try_register_alias(
                    definition.alias_type,
                    definition.target_type,
                    definition.alias_name,
                    definition.target_name,
                    &definition.metadata,
                    self.allow_definition_overriding,
                )?;
            }
        }

        Ok(())
    }

    fn register_unconditional_components(
        component_definitions: &[TypedComponentDefinition],
        allow_definition_overriding: bool,
    ) -> Result<NamedComponentDefinitionMap, ComponentDefinitionRegistryError> {
        let mut definition_map = NamedComponentDefinitionMap::default();

        for definition in component_definitions
            .iter()
            .filter(|definition| definition.condition.is_none())
        {
            definition_map.try_register_component(
                definition.target,
                definition.target_name,
                &definition.metadata,
                allow_definition_overriding,
            )?;
        }

        Ok(definition_map)
    }

    fn register_unconditional_aliases(
        &mut self,
        alias_definitions: &[ComponentAliasDefinition],
    ) -> Result<(), ComponentDefinitionRegistryError> {
        for definition in alias_definitions
            .iter()
            .filter(|definition| definition.condition.is_none())
        {
            self.definition_map.try_register_alias(
                definition.alias_type,
                definition.target_type,
                definition.alias_name,
                definition.target_name,
                &definition.metadata,
                self.allow_definition_overriding,
            )?;
        }

        Ok(())
    }
}

impl ComponentDefinitionRegistry for StaticComponentDefinitionRegistry {
    #[inline]
    fn register_component(
        &mut self,
        target: TypeId,
        target_name: &str,
        metadata: &ComponentMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError> {
        self.definition_map.try_register_component(
            target,
            target_name,
            metadata,
            self.allow_definition_overriding,
        )
    }

    #[inline]
    fn register_alias(
        &mut self,
        alias_type: TypeId,
        target_type: TypeId,
        alias_name: &str,
        target_name: &str,
        metadata: &ComponentAliasMetadata,
    ) -> Result<(), ComponentDefinitionRegistryError> {
        self.definition_map.try_register_alias(
            alias_type,
            target_type,
            alias_name,
            target_name,
            metadata,
            self.allow_definition_overriding,
        )
    }

    #[inline]
    fn components_by_type(&self, type_id: TypeId) -> Vec<ComponentDefinition> {
        self.definition_map.components_by_type(type_id)
    }

    #[inline]
    fn component_by_name(&self, name: &str) -> Option<ComponentDefinition> {
        self.definition_map.component_by_name(name)
    }

    #[inline]
    fn primary_component(&self, type_id: TypeId) -> Option<ComponentDefinition> {
        self.definition_map.primary_component(type_id)
    }

    #[inline]
    fn is_registered(&self, type_id: TypeId) -> bool {
        <Self as ComponentDefinitionRegistryFacade>::is_registered(self, type_id)
    }

    #[inline]
    fn is_name_registered(&self, name: &str) -> bool {
        <Self as ComponentDefinitionRegistryFacade>::is_name_registered(self, name)
    }

    #[inline]
    fn all_definitions(&self) -> FxHashMap<TypeId, Vec<ComponentDefinition>> {
        self.definition_map.all_definitions()
    }
}

impl ComponentDefinitionRegistryFacade for StaticComponentDefinitionRegistry {
    #[inline]
    fn is_registered(&self, target: TypeId) -> bool {
        self.definition_map.is_registered(target)
    }

    #[inline]
    fn is_name_registered(&self, name: &str) -> bool {
        self.definition_map.is_name_registered(name)
    }
}

mod registry {
    use crate::component_registry::ComponentDefinitionRegistryError;
    use crate::component_registry::{
        ComponentAliasMetadata, ComponentDefinition, ComponentMetadata,
    };
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

        pub(super) fn components_by_type(&self, type_id: TypeId) -> Vec<ComponentDefinition> {
            self.definitions.get(&type_id).cloned().unwrap_or_default()
        }

        pub(super) fn primary_component(&self, type_id: TypeId) -> Option<ComponentDefinition> {
            self.definitions.get(&type_id).and_then(|definitions| {
                (if definitions.len() == 1 {
                    definitions.get(0)
                } else {
                    definitions.iter().find(|definition| definition.is_primary)
                })
                .cloned()
            })
        }

        pub(super) fn try_register_alias(
            &mut self,
            alias_type: TypeId,
            target_type: TypeId,
            alias_name: &str,
            target_name: &str,
            metadata: &ComponentAliasMetadata,
            allow_definition_overriding: bool,
        ) -> Result<(), ComponentDefinitionRegistryError> {
            let mut target_definitions = self
                .definitions
                .get(&target_type)
                .ok_or(ComponentDefinitionRegistryError::MissingBaseComponent {
                    alias_type: alias_name.to_string(),
                    target_type: target_name.to_string(),
                })
                .cloned()?;

            // this should not be possible, since we don't allow removing definitions, but better
            // be safe
            if target_definitions.is_empty() {
                return Err(ComponentDefinitionRegistryError::MissingBaseComponent {
                    alias_type: alias_name.to_string(),
                    target_type: target_name.to_string(),
                });
            }

            // if we're registering a primary alias, there needs to be a single target
            if metadata.is_primary && target_definitions.len() > 1 {
                return Err(
                    ComponentDefinitionRegistryError::DuplicatePrimaryComponent {
                        alias_type: alias_name.to_string(),
                        target_type: target_name.to_string(),
                    },
                );
            }

            // should run once due to above anyway
            for definition in &mut target_definitions {
                definition.is_primary = metadata.is_primary;
                definition.scope_name = metadata.scope_name.clone();
                definition.cast = metadata.cast;
            }

            if let Some(name) = &metadata.name {
                if !allow_definition_overriding && self.names.contains_key(name) {
                    return Err(ComponentDefinitionRegistryError::DuplicateComponentName(
                        name.clone(),
                    ));
                }

                // name should point to a single definition; the target, in general, should be a
                // single concrete type, but in theory, we can point an alias to an alias and let it
                // resolve later somehow
                if target_definitions.len() > 1 {
                    return Err(ComponentDefinitionRegistryError::CannotRegisterNamedAliasForAmbiguousTarget {
                        alias_type: alias_name.to_string(),
                        target_type: target_name.to_string(),
                        name: name.clone(),
                    });
                }

                for definition in &mut target_definitions {
                    definition.names.insert(name.clone());
                }
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

                if let Some(name) = &metadata.name {
                    // at this point target_definitions should have only one definition
                    debug_assert_eq!(target_definitions.len(), 1);

                    self.names
                        .insert(name.clone(), (alias_type, alias_definitions.len()));
                }

                alias_definitions.append(&mut target_definitions);
            } else {
                if let Some(name) = &metadata.name {
                    self.names.insert(name.clone(), (alias_type, 0));
                }

                self.definitions.insert(alias_type, target_definitions);
            }

            Ok(())
        }

        pub(super) fn try_register_component(
            &mut self,
            target: TypeId,
            target_name: &str,
            metadata: &ComponentMetadata,
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
                names: metadata.names.clone(),
                is_primary: false,
                scope_name: metadata.scope_name.clone(),
                resolved_type_id: target,
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

        #[inline]
        pub(super) fn is_registered(&self, target: TypeId) -> bool {
            self.definitions
                .get(&target)
                .map(|definitions| !definitions.is_empty())
                .unwrap_or(false)
        }

        #[inline]
        pub(super) fn is_name_registered(&self, name: &str) -> bool {
            self.names.contains_key(name)
        }

        #[inline]
        pub(super) fn all_definitions(&self) -> FxHashMap<TypeId, Vec<ComponentDefinition>> {
            self.definitions.clone()
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::component_registry::registry::NamedComponentDefinitionMap;
        use crate::component_registry::ComponentDefinitionRegistryError;
        use crate::component_registry::{ComponentAliasMetadata, ComponentMetadata};
        use crate::instance_provider::{
            ComponentInstanceAnyPtr, ComponentInstanceProvider, ComponentInstanceProviderError,
            ComponentInstancePtr,
        };
        use std::any::{Any, TypeId};

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

        fn create_metadata() -> (ComponentMetadata, TypeId) {
            (
                ComponentMetadata {
                    names: ["name".to_string()].into_iter().collect(),
                    scope_name: "".to_string(),
                    constructor,
                    cast,
                },
                TypeId::of::<i8>(),
            )
        }

        #[test]
        fn should_reject_alias_name_for_ambiguous_target() {
            let definition = ComponentMetadata {
                names: Default::default(),
                scope_name: "".to_string(),
                constructor,
                cast,
            };
            let alias_id_1 = TypeId::of::<u8>();
            let alias_id_2 = TypeId::of::<u16>();
            let target_id_1 = TypeId::of::<i16>();
            let target_id_2 = TypeId::of::<u16>();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(target_id_1, "", &definition, false)
                .unwrap();
            registry
                .try_register_component(target_id_2, "", &definition, false)
                .unwrap();
            registry
                .try_register_alias(
                    alias_id_1,
                    target_id_1,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: false,
                        name: None,
                        scope_name: "".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();
            registry
                .try_register_alias(
                    alias_id_1,
                    target_id_2,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: false,
                        name: None,
                        scope_name: "".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();

            assert!(matches!(
                registry
                    .try_register_alias(
                        alias_id_2,
                        alias_id_1,
                        "",
                        "",
                        &ComponentAliasMetadata {
                            is_primary: false,
                            name: Some("alias_name".to_string()),
                            scope_name: "".to_string(),
                            cast,
                        },
                        false,
                    )
                    .unwrap_err(),
                ComponentDefinitionRegistryError::CannotRegisterNamedAliasForAmbiguousTarget { .. }
            ));
        }

        #[test]
        fn should_reject_primary_alias_for_ambiguous_target() {
            let definition = ComponentMetadata {
                names: Default::default(),
                scope_name: "".to_string(),
                constructor,
                cast,
            };
            let alias_id_1 = TypeId::of::<u8>();
            let alias_id_2 = TypeId::of::<u16>();
            let target_id_1 = TypeId::of::<i16>();
            let target_id_2 = TypeId::of::<u16>();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(target_id_1, "", &definition, false)
                .unwrap();
            registry
                .try_register_component(target_id_2, "", &definition, false)
                .unwrap();
            registry
                .try_register_alias(
                    alias_id_1,
                    target_id_1,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: false,
                        name: None,
                        scope_name: "".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();
            registry
                .try_register_alias(
                    alias_id_1,
                    target_id_2,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: false,
                        name: None,
                        scope_name: "".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();

            assert!(matches!(
                registry
                    .try_register_alias(
                        alias_id_2,
                        alias_id_1,
                        "",
                        "",
                        &ComponentAliasMetadata {
                            is_primary: true,
                            name: None,
                            scope_name: "".to_string(),
                            cast,
                        },
                        false,
                    )
                    .unwrap_err(),
                ComponentDefinitionRegistryError::DuplicatePrimaryComponent { .. }
            ));
        }

        #[test]
        fn should_register_definition() {
            let (definition, id) = create_metadata();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, "", &definition, false)
                .unwrap();

            assert_eq!(registry.components_by_type(id)[0].names, definition.names);
            assert_eq!(
                registry.component_by_name("name").unwrap().names,
                definition.names
            );
            assert!(registry.is_registered(id));
            assert!(registry.is_name_registered("name"));
        }

        #[test]
        fn should_not_register_duplicate_name() {
            let (definition, id) = create_metadata();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, "", &definition, false)
                .unwrap();

            assert!(matches!(
                registry
                    .try_register_component(id, "", &definition, false)
                    .unwrap_err(),
                ComponentDefinitionRegistryError::DuplicateComponentName(..)
            ));
        }

        #[test]
        fn should_override_duplicate_name() {
            let (definition, id) = create_metadata();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, "", &definition, true)
                .unwrap();
            registry
                .try_register_component(id, "", &definition, true)
                .unwrap();
        }

        #[test]
        fn should_register_alias() {
            let (definition, id) = create_metadata();
            let alias_id = TypeId::of::<u8>();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, "", &definition, false)
                .unwrap();
            registry
                .try_register_alias(
                    alias_id,
                    id,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: false,
                        name: None,
                        scope_name: "".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();

            assert_eq!(
                registry.components_by_type(alias_id)[0].names,
                definition.names
            );
        }

        #[test]
        fn should_register_alias_scope() {
            let (definition, id) = create_metadata();
            let alias_id = TypeId::of::<u8>();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, "", &definition, false)
                .unwrap();
            registry
                .try_register_alias(
                    alias_id,
                    id,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: false,
                        name: None,
                        scope_name: "scope".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();

            assert_eq!(registry.components_by_type(alias_id)[0].scope_name, "scope");
        }

        #[test]
        fn should_register_alias_name() {
            let (definition, id) = create_metadata();
            let alias_id = TypeId::of::<u8>();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, "", &definition, false)
                .unwrap();
            registry
                .try_register_alias(
                    alias_id,
                    id,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: false,
                        name: Some("alias".to_string()),
                        scope_name: "".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();

            assert!(registry.component_by_name("alias").is_some());
        }

        #[test]
        fn should_reject_duplicate_primary_alias() {
            let (definition, id) = create_metadata();
            let alias_id = TypeId::of::<u8>();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, "", &definition, false)
                .unwrap();
            registry
                .try_register_alias(
                    alias_id,
                    id,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: true,
                        name: None,
                        scope_name: "".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();

            assert!(matches!(
                registry
                    .try_register_alias(
                        alias_id,
                        id,
                        "",
                        "",
                        &ComponentAliasMetadata {
                            is_primary: true,
                            name: None,
                            scope_name: "".to_string(),
                            cast
                        },
                        false
                    )
                    .unwrap_err(),
                ComponentDefinitionRegistryError::DuplicatePrimaryComponent { .. }
            ));
        }

        #[test]
        fn should_return_primary_single_definition() {
            let (definition, id) = create_metadata();
            let alias_id = TypeId::of::<u8>();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id, "", &definition, false)
                .unwrap();
            registry
                .try_register_alias(
                    alias_id,
                    id,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: false,
                        name: None,
                        scope_name: "".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();

            assert!(registry.primary_component(alias_id).is_some());
        }

        #[test]
        fn should_return_explicit_primary_definition() {
            let (definition, id_1) = create_metadata();
            let id_2 = TypeId::of::<u16>();
            let alias_id = TypeId::of::<u8>();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id_1, "", &definition, false)
                .unwrap();
            registry
                .try_register_component(id_2, "", &definition, true)
                .unwrap();
            registry
                .try_register_alias(
                    alias_id,
                    id_1,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: false,
                        name: None,
                        scope_name: "".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();
            registry
                .try_register_alias(
                    alias_id,
                    id_2,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: true,
                        name: None,
                        scope_name: "".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();

            assert!(registry.primary_component(alias_id).is_some());
        }

        #[test]
        fn should_not_return_unknown_primary_definition() {
            let (definition, id_1) = create_metadata();
            let id_2 = TypeId::of::<u16>();
            let alias_id = TypeId::of::<u8>();

            let mut registry = NamedComponentDefinitionMap::default();
            registry
                .try_register_component(id_1, "", &definition, false)
                .unwrap();
            registry
                .try_register_component(id_2, "", &definition, true)
                .unwrap();
            registry
                .try_register_alias(
                    alias_id,
                    id_1,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: false,
                        name: None,
                        scope_name: "".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();
            registry
                .try_register_alias(
                    alias_id,
                    id_2,
                    "",
                    "",
                    &ComponentAliasMetadata {
                        is_primary: false,
                        name: None,
                        scope_name: "".to_string(),
                        cast,
                    },
                    false,
                )
                .unwrap();

            assert!(registry.primary_component(alias_id).is_none());
        }
    }
}

#[doc(hidden)]
pub mod internal {
    use crate::component_registry::conditional::ComponentCondition;
    use crate::component_registry::{ComponentAliasMetadata, ComponentMetadata};
    use inventory::collect;
    pub use inventory::submit;
    use std::any::TypeId;

    #[derive(Clone)]
    pub struct TypedComponentDefinition {
        pub target: TypeId,
        pub target_name: &'static str,
        pub condition: Option<ComponentCondition>,
        pub priority: i8,
        pub metadata: ComponentMetadata,
    }

    pub struct ComponentDefinitionRegisterer {
        pub register: fn() -> TypedComponentDefinition,
    }

    #[derive(Clone)]
    pub struct ComponentAliasDefinition {
        pub alias_type: TypeId,
        pub target_type: TypeId,
        pub alias_name: &'static str,
        pub target_name: &'static str,
        pub condition: Option<ComponentCondition>,
        pub priority: i8,
        pub metadata: ComponentAliasMetadata,
    }

    pub struct ComponentAliasRegisterer {
        pub register: fn() -> ComponentAliasDefinition,
    }

    collect!(ComponentDefinitionRegisterer);
    collect!(ComponentAliasRegisterer);
}

#[cfg(test)]
//noinspection DuplicatedCode
mod tests {
    use crate::component::{Component, ComponentDowncast, Injectable};
    use crate::component_registry::conditional::{
        ComponentDefinitionRegistryFacade, SimpleContextFactory,
    };
    use crate::component_registry::ComponentDefinitionRegistryError;
    use crate::component_registry::{
        ComponentDefinition, ComponentDefinitionRegistry, ComponentMetadata,
        StaticComponentDefinitionRegistry, TypedComponentDefinitionRegistry,
    };
    use crate::instance_provider::{
        ComponentInstanceAnyPtr, ComponentInstanceProvider, ComponentInstanceProviderError,
        ComponentInstancePtr,
    };
    use std::any::{Any, TypeId};

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

    fn test_cast(
        instance: ComponentInstanceAnyPtr,
    ) -> Result<Box<dyn Any>, ComponentInstanceAnyPtr> {
        TestComponent::downcast(instance).map(|p| Box::new(p) as Box<dyn Any>)
    }

    #[test]
    fn should_register_definition() {
        let mut registry =
            StaticComponentDefinitionRegistry::new(false, &SimpleContextFactory::default())
                .unwrap();
        registry
            .register_component_typed::<TestComponent>(&ComponentMetadata {
                names: ["a".to_string()].into_iter().collect(),
                scope_name: "".to_string(),
                constructor: test_constructor,
                cast: test_cast,
            })
            .unwrap();

        assert!(!registry
            .components_by_type_typed::<TestComponent>()
            .is_empty());
        assert!(TypedComponentDefinitionRegistry::is_registered_typed::<
            TestComponent,
        >(&registry));
        assert!(ComponentDefinitionRegistryFacade::is_registered(
            &registry,
            TypeId::of::<TestComponent>()
        ));
        assert!(ComponentDefinitionRegistry::is_name_registered(
            &registry, "a"
        ));
    }

    #[test]
    fn should_not_register_duplicate_name() {
        let definition = ComponentDefinition {
            names: ["name".to_string()].into_iter().collect(),
            is_primary: false,
            scope_name: "".to_string(),
            resolved_type_id: TypeId::of::<TestComponent>(),
            constructor: test_constructor,
            cast: test_cast,
        };

        let mut registry =
            StaticComponentDefinitionRegistry::new(false, &SimpleContextFactory::default())
                .unwrap();
        registry
            .register_component_typed::<TestComponent>(&ComponentMetadata {
                names: definition.names.clone(),
                scope_name: "".to_string(),
                constructor: test_constructor,
                cast: test_cast,
            })
            .unwrap();

        assert_eq!(
            registry
                .register_component_typed::<TestComponent>(&ComponentMetadata {
                    names: definition.names,
                    scope_name: "".to_string(),
                    constructor: test_constructor,
                    cast: test_cast,
                })
                .unwrap_err(),
            ComponentDefinitionRegistryError::DuplicateComponentName("name".to_string())
        );
    }

    #[test]
    fn should_override_duplicate_name() {
        let mut registry =
            StaticComponentDefinitionRegistry::new(true, &SimpleContextFactory::default()).unwrap();
        registry
            .register_component_typed::<TestComponent>(&ComponentMetadata {
                names: ["name".to_string()].into_iter().collect(),
                scope_name: "".to_string(),
                constructor: test_constructor,
                cast: test_cast,
            })
            .unwrap();
        registry
            .register_component_typed::<TestComponent>(&ComponentMetadata {
                names: ["name2".to_string()].into_iter().collect(),
                scope_name: "".to_string(),
                constructor: test_constructor,
                cast: test_cast,
            })
            .unwrap();

        registry.component_by_name("name2").unwrap();
    }
}
