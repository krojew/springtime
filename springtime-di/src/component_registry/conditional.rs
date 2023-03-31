//! Conditional component definition registration support.

use crate::component_registry::{ComponentAliasMetadata, ComponentMetadata};
use std::any::TypeId;

/// A read-only facade of a [ComponentDefinitionRegistry](super::ComponentDefinitionRegistry) safe
/// to use in registration conditions.
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
