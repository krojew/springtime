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
    Component(&'a ComponentMetadata),
    Alias(&'a ComponentAliasMetadata),
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
pub fn registered_component<T: Injectable>(
    context: &dyn Context,
    _metadata: ConditionMetadata,
) -> bool {
    context.registry().is_registered(TypeId::of::<T>())
}

/// Simple condition returning true if the given type is not yet registered.
pub fn unregistered_component<T: Injectable>(
    context: &dyn Context,
    metadata: ConditionMetadata,
) -> bool {
    !registered_component::<T>(context, metadata)
}

#[cfg(test)]
mod tests {
    use crate::component::Injectable;
    use crate::component_registry::conditional::{
        registered_component, unregistered_component, ConditionMetadata,
        MockComponentDefinitionRegistryFacade, SimpleContext,
    };
    use crate::component_registry::ComponentAliasMetadata;
    use crate::instance_provider::ComponentInstanceAnyPtr;
    use mockall::predicate::*;
    use mockall::Sequence;
    use std::any::TypeId;

    struct TestComponent;

    impl Injectable for TestComponent {}

    unsafe fn test_cast(
        instance: ComponentInstanceAnyPtr,
        _result: *mut (),
    ) -> Result<(), ComponentInstanceAnyPtr> {
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
        let metadata = ConditionMetadata::Alias(&ComponentAliasMetadata {
            is_primary: false,
            cast: test_cast,
        });

        assert!(registered_component::<TestComponent>(&context, metadata));
        assert!(!unregistered_component::<TestComponent>(&context, metadata));
        assert!(!registered_component::<TestComponent>(&context, metadata));
        assert!(unregistered_component::<TestComponent>(&context, metadata));
    }
}
