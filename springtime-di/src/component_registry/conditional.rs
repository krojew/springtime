//! Conditional component definition registration support.

use crate::component::Injectable;
use crate::component_registry::{ComponentAliasMetadata, ComponentMetadata};
#[cfg(test)]
use mockall::automock;
use std::any::TypeId;

/// A read-only facade of a [ComponentDefinitionRegistry](super::ComponentDefinitionRegistry) safe
/// to use in registration conditions.
#[cfg_attr(test, automock)]
pub trait ComponentDefinitionRegistryFacade {
    /// Checks if given type is present in this registry.
    fn is_registered(&self, target: TypeId) -> bool;

    /// Checks if there's a definition with given name.
    fn is_name_registered(&self, name: &str) -> bool;
}

/// Context information for use by condition implementations.
pub trait Context {
    /// Returns the registry for which the conditional evaluation is taking place.
    fn registry(&self) -> &dyn ComponentDefinitionRegistryFacade;
}

/// Factory for contexts for conditional component registration.
pub trait ContextFactory {
    /// Creates a new context when starting evaluation.
    fn create_context<'a>(
        &self,
        registry: &'a dyn ComponentDefinitionRegistryFacade,
    ) -> Box<dyn Context + 'a>;
}

/// Metadata for the entity which is currently evaluated for registration.
#[derive(Clone, Debug, Copy)]
pub enum ConditionMetadata<'a> {
    Component {
        type_id: TypeId,
        metadata: &'a ComponentMetadata,
    },
    Alias {
        alias_type: TypeId,
        target_type: TypeId,
        metadata: &'a ComponentAliasMetadata,
    },
}

/// Registration condition which should pass to let given [ConditionMetadata] be registered.
pub type ComponentCondition = fn(context: &dyn Context, metadata: ConditionMetadata) -> bool;

struct SimpleContext<'a> {
    registry: &'a dyn ComponentDefinitionRegistryFacade,
}

impl Context for SimpleContext<'_> {
    fn registry(&self) -> &dyn ComponentDefinitionRegistryFacade {
        self.registry
    }
}

/// Factory producing contexts containing only the necessary data and noting more.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct SimpleContextFactory;

impl ContextFactory for SimpleContextFactory {
    fn create_context<'a>(
        &self,
        registry: &'a dyn ComponentDefinitionRegistryFacade,
    ) -> Box<dyn Context + 'a> {
        Box::new(SimpleContext { registry })
    }
}

/// Simple condition returning true if the given type is already registered.
pub fn registered_component<T: Injectable + ?Sized>(
    context: &dyn Context,
    _metadata: ConditionMetadata,
) -> bool {
    context.registry().is_registered(TypeId::of::<T>())
}

/// Simple condition returning true if the given type is not registered yet.
pub fn unregistered_component<T: Injectable + ?Sized>(
    context: &dyn Context,
    metadata: ConditionMetadata,
) -> bool {
    !registered_component::<T>(context, metadata)
}

/// Returns true if no given name is already registered.
pub fn unregistered_name(context: &dyn Context, metadata: ConditionMetadata) -> bool {
    let registry = context.registry();
    match metadata {
        ConditionMetadata::Component {
            metadata: ComponentMetadata { names, .. },
            ..
        } => !names.iter().any(|name| registry.is_name_registered(name)),
        ConditionMetadata::Alias { .. } => true,
    }
}

#[cfg(test)]
mod tests {
    use crate::component::Injectable;
    use crate::component_registry::conditional::{
        registered_component, unregistered_component, unregistered_name, ConditionMetadata,
        MockComponentDefinitionRegistryFacade, SimpleContext,
    };
    use crate::component_registry::{ComponentAliasMetadata, ComponentMetadata};
    use crate::instance_provider::ComponentInstanceProviderError;
    use crate::instance_provider::{ComponentInstanceAnyPtr, ComponentInstanceProvider};
    use mockall::predicate::*;
    use mockall::Sequence;
    use std::any::{Any, TypeId};

    struct TestComponent;

    impl Injectable for TestComponent {}

    fn test_constructor(
        _instance_provider: &mut dyn ComponentInstanceProvider,
    ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
        Err(ComponentInstanceProviderError::NoNamedInstance(
            "".to_string(),
        ))
    }

    fn test_cast(
        instance: ComponentInstanceAnyPtr,
    ) -> Result<Box<dyn Any>, ComponentInstanceAnyPtr> {
        Err(instance)
    }

    #[test]
    fn should_check_for_component_existence() {
        let mut seq = Sequence::new();

        let mut registry = MockComponentDefinitionRegistryFacade::new();
        registry
            .expect_is_registered()
            .with(eq(TypeId::of::<TestComponent>()))
            .times(2)
            .in_sequence(&mut seq)
            .return_const(true);
        registry
            .expect_is_registered()
            .with(eq(TypeId::of::<TestComponent>()))
            .times(2)
            .in_sequence(&mut seq)
            .return_const(false);

        let context = SimpleContext {
            registry: &registry,
        };
        let metadata = ComponentAliasMetadata {
            is_primary: false,
            scope: None,
            cast: test_cast,
        };
        let metadata = ConditionMetadata::Alias {
            alias_type: TypeId::of::<i8>(),
            target_type: TypeId::of::<TestComponent>(),
            metadata: &metadata,
        };

        assert!(registered_component::<TestComponent>(&context, metadata));
        assert!(!unregistered_component::<TestComponent>(&context, metadata));
        assert!(!registered_component::<TestComponent>(&context, metadata));
        assert!(unregistered_component::<TestComponent>(&context, metadata));
    }

    #[test]
    fn should_check_for_name_existence() {
        let mut registry = MockComponentDefinitionRegistryFacade::new();
        registry
            .expect_is_name_registered()
            .with(eq("n1"))
            .times(1)
            .return_const(true);
        registry
            .expect_is_name_registered()
            .with(eq("n2"))
            .times(1)
            .return_const(false);

        let context = SimpleContext {
            registry: &registry,
        };

        let metadata = ComponentMetadata {
            names: ["n2".to_string(), "n1".to_string()].into_iter().collect(),
            scope: "".to_string(),
            constructor: test_constructor,
            cast: test_cast,
        };
        let metadata = ConditionMetadata::Component {
            type_id: TypeId::of::<TestComponent>(),
            metadata: &metadata,
        };

        assert!(!unregistered_name(&context, metadata));

        let metadata = ComponentAliasMetadata {
            is_primary: false,
            scope: None,
            cast: test_cast,
        };
        let metadata = ConditionMetadata::Alias {
            alias_type: TypeId::of::<i8>(),
            target_type: TypeId::of::<TestComponent>(),
            metadata: &metadata,
        };

        assert!(unregistered_name(&context, metadata));
    }
}
