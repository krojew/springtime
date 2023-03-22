use crate::component::Component;
use crate::component_registry::internal::ComponentDefinitionRegisterer;
use crate::error::ComponentDefinitionRegistryError;
use fxhash::FxHashMap;
use std::any::TypeId;

/// Definition for a [Component](Component) registered in a definition registry.
#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Hash, Debug, Default)]
pub struct ComponentDefinition {
    /// Each component has its own name, which can be used to request a specific instance.
    /// Derive-based components have their name generated from type name by converting it to snake
    /// case.
    pub name: String,
    /// With multiple components registered for a given type, one of them can be marked as primary
    /// and returned when requesting a single instance.
    pub is_primary: bool,
}

/// A registry of component definitions which can be used when requesting instances via a
/// [ComponentInstanceProvider](crate::component::ComponentInstanceProvider).
pub trait ComponentDefinitionRegistry {
    /// Adds a new definition for a given type. Note: handling of duplicate component names is
    /// registry-dependent.
    fn register_component<T: Component + 'static>(
        &mut self,
        definition: ComponentDefinition,
    ) -> Result<(), ComponentDefinitionRegistryError>;

    /// Returns all registered definitions for a given type.
    fn components_by_type<T: Component + 'static>(&self) -> Option<Vec<ComponentDefinition>>;

    /// Returns a definition for a given type, with given name.
    fn component_by_name<T: Component + 'static>(&self, name: &str) -> Option<ComponentDefinition>;
}

/// Registry of component definitions initialized from statically registered definitions.
pub struct StaticComponentDefinitionRegistry {
    definitions: FxHashMap<TypeId, Vec<ComponentDefinition>>,
    allow_definition_overriding: bool,
}

impl StaticComponentDefinitionRegistry {
    pub fn new(
        allow_definition_overriding: bool,
    ) -> Result<Self, ComponentDefinitionRegistryError> {
        let mut definitions = FxHashMap::default();
        for registerer in inventory::iter::<ComponentDefinitionRegisterer> {
            let definition = (registerer.register)();
            let registry: &mut Vec<ComponentDefinition> =
                definitions.entry(definition.target).or_default();

            Self::try_register_component(
                registry,
                definition.definition,
                allow_definition_overriding,
            )?;
        }

        Ok(Self {
            definitions,
            allow_definition_overriding,
        })
    }

    fn try_register_component(
        registry: &mut Vec<ComponentDefinition>,
        definition: ComponentDefinition,
        allow_definition_overriding: bool,
    ) -> Result<(), ComponentDefinitionRegistryError> {
        if let Some(entry) = registry
            .iter_mut()
            .find(|entry| entry.name == definition.name)
        {
            if !allow_definition_overriding {
                return Err(ComponentDefinitionRegistryError::DuplicateName(
                    definition.name,
                ));
            }

            *entry = definition;
        } else {
            registry.push(definition);
        }

        Ok(())
    }
}

impl ComponentDefinitionRegistry for StaticComponentDefinitionRegistry {
    fn register_component<T: Component + 'static>(
        &mut self,
        definition: ComponentDefinition,
    ) -> Result<(), ComponentDefinitionRegistryError> {
        let registry = self.definitions.entry(TypeId::of::<T>()).or_default();
        Self::try_register_component(registry, definition, self.allow_definition_overriding)
    }

    fn components_by_type<T: Component + 'static>(&self) -> Option<Vec<ComponentDefinition>> {
        self.definitions.get(&TypeId::of::<T>()).cloned()
    }

    fn component_by_name<T: Component + 'static>(&self, name: &str) -> Option<ComponentDefinition> {
        self.definitions
            .get(&TypeId::of::<T>())
            .and_then(|definitions| definitions.iter().find(|entry| entry.name == name))
            .cloned()
    }
}

#[doc(hidden)]
pub mod internal {
    use crate::component_registry::ComponentDefinition;
    use std::any::TypeId;

    pub use inventory::submit;

    pub struct ComponentDefinitionRegisterer {
        pub register: fn() -> TypedComponentDefinition,
    }

    pub struct TypedComponentDefinition {
        pub target: TypeId,
        pub definition: ComponentDefinition,
    }

    inventory::collect!(ComponentDefinitionRegisterer);
}

#[cfg(test)]
mod tests {
    use crate::component::{Component, ComponentInstanceProvider};
    use crate::component_registry::{
        ComponentDefinition, ComponentDefinitionRegistry, StaticComponentDefinitionRegistry,
    };
    use crate::error::{ComponentDefinitionRegistryError, ComponentInstanceProviderError};

    struct TestComponent;

    impl Component for TestComponent {
        fn create<CIP: ComponentInstanceProvider>(
            _instance_provider: &CIP,
        ) -> Result<Self, ComponentInstanceProviderError>
        where
            Self: Sized,
        {
            Ok(TestComponent)
        }
    }

    #[test]
    fn should_register_definition() {
        let mut registry = StaticComponentDefinitionRegistry::new(false).unwrap();
        registry
            .register_component::<TestComponent>(ComponentDefinition {
                name: "name".to_string(),
                is_primary: false,
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
            name: "name".to_string(),
            is_primary: false,
        };

        let mut registry = StaticComponentDefinitionRegistry::new(false).unwrap();
        registry
            .register_component::<TestComponent>(definition.clone())
            .unwrap();

        assert_eq!(
            registry
                .register_component::<TestComponent>(definition.clone())
                .unwrap_err(),
            ComponentDefinitionRegistryError::DuplicateName(definition.name)
        );
    }

    #[test]
    fn should_override_duplicate_name() {
        let definition1 = ComponentDefinition {
            name: "name".to_string(),
            is_primary: false,
        };

        let definition2 = ComponentDefinition {
            name: "name".to_string(),
            is_primary: true,
        };

        let mut registry = StaticComponentDefinitionRegistry::new(true).unwrap();
        registry
            .register_component::<TestComponent>(definition1.clone())
            .unwrap();
        registry
            .register_component::<TestComponent>(definition2)
            .unwrap();

        assert!(
            registry
                .component_by_name::<TestComponent>(&definition1.name)
                .unwrap()
                .is_primary
        );
    }
}
