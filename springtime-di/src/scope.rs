//! Component instances are contained in [Scope]s - containers which decide when to reuse or create
//! an instance. There's a global one for singletons, but there also can be other, specialized ones.
//! Some can be simple, like [PrototypeScope], while other can be quite complex and depend on
//! external factors, e.g. tying the lifetime of instances to web sessions.
//!
//! Note: scope resolution happens at component instantiation time, which can lead to unexpected
//! consequences if incompatible scopes are mixed together, e.g. a [singleton](SINGLETON) component
//! can depend on a [prototype](PROTOTYPE) one. In such case when creating the singleton, a new
//! instance of the dependency will be created, since it's a prototype, but then that single
//! instance will live as long as the singleton lives.

use crate::component_registry::ComponentDefinition;
use crate::instance_provider::ComponentInstanceAnyPtr;
use fxhash::FxHashMap;
#[cfg(test)]
use mockall::automock;
use std::any::TypeId;

#[cfg(not(feature = "threadsafe"))]
pub type ScopePtr = Box<dyn Scope>;
#[cfg(feature = "threadsafe")]
pub type ScopePtr = Box<dyn Scope + Send + Sync>;

/// Name of the [SingletonScope].
pub const SINGLETON: &str = "SINGLETON";

/// Name of the [PrototypeScope].
pub const PROTOTYPE: &str = "PROTOTYPE";

/// A scope containing component instances. See module documentation for information on scopes.
#[cfg_attr(test, automock)]
pub trait Scope {
    /// Gets an instance requested for the given definition, if available in this scope.
    fn instance(&self, definition: &ComponentDefinition) -> Option<ComponentInstanceAnyPtr>;

    /// Stores given instance in the scope. The scope might not support storing instances and ignore
    /// it.
    fn store_instance(
        &mut self,
        definition: &ComponentDefinition,
        instance: ComponentInstanceAnyPtr,
    );
}

/// Scope for instances shared between components. Stateless components are good candidates to be
/// stored in the singleton scope.
#[derive(Default)]
pub struct SingletonScope {
    instances: FxHashMap<TypeId, ComponentInstanceAnyPtr>,
}

impl Scope for SingletonScope {
    #[inline]
    fn instance(&self, definition: &ComponentDefinition) -> Option<ComponentInstanceAnyPtr> {
        self.instances.get(&definition.resolved_type_id).cloned()
    }

    #[inline]
    fn store_instance(
        &mut self,
        definition: &ComponentDefinition,
        instance: ComponentInstanceAnyPtr,
    ) {
        self.instances.insert(definition.resolved_type_id, instance);
    }
}

/// A scope which creates a new instance of a given component on each request. Stateful components
/// usually should be stored in a prototype scope.
#[derive(Default, Copy, Clone, Eq, PartialEq)]
pub struct PrototypeScope;

impl Scope for PrototypeScope {
    #[inline]
    fn instance(&self, _definition: &ComponentDefinition) -> Option<ComponentInstanceAnyPtr> {
        None
    }

    #[inline]
    fn store_instance(
        &mut self,
        _definition: &ComponentDefinition,
        _instance: ComponentInstanceAnyPtr,
    ) {
    }
}

/// Factory for custom [Scope]s.
#[cfg_attr(test, automock)]
pub trait ScopeFactory {
    fn create_scope(&self) -> ScopePtr;
}

#[derive(Copy, Clone, Eq, PartialEq, Default)]
pub struct SingletonScopeFactory;

impl ScopeFactory for SingletonScopeFactory {
    fn create_scope(&self) -> ScopePtr {
        Box::<SingletonScope>::default()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Default)]
pub struct PrototypeScopeFactory;

impl ScopeFactory for PrototypeScopeFactory {
    fn create_scope(&self) -> ScopePtr {
        Box::<PrototypeScope>::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::component_registry::ComponentDefinition;
    use crate::instance_provider::ComponentInstanceProviderError;
    use crate::instance_provider::{
        ComponentInstanceAnyPtr, ComponentInstanceProvider, ComponentInstancePtr,
    };
    use crate::scope::{PrototypeScopeFactory, ScopeFactory, SingletonScopeFactory};
    use std::any::{Any, TypeId};

    fn test_constructor(
        _instance_provider: &mut dyn ComponentInstanceProvider,
    ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError> {
        Err(ComponentInstanceProviderError::IncompatibleComponent(
            TypeId::of::<i8>(),
        ))
    }

    fn test_cast(
        instance: ComponentInstanceAnyPtr,
    ) -> Result<Box<dyn Any>, ComponentInstanceAnyPtr> {
        Err(instance)
    }

    fn create_definition() -> ComponentDefinition {
        ComponentDefinition {
            names: Default::default(),
            is_primary: false,
            scope: "".to_string(),
            resolved_type_id: TypeId::of::<u8>(),
            constructor: test_constructor,
            cast: test_cast,
        }
    }

    #[test]
    fn should_support_singletons() {
        let definition = create_definition();
        let factory = SingletonScopeFactory;
        let mut scope = factory.create_scope();

        let instance = ComponentInstancePtr::new(0) as ComponentInstanceAnyPtr;
        scope.store_instance(&definition, instance.clone());

        assert!(scope.instance(&definition).is_some());
    }

    #[test]
    fn should_support_prototypes() {
        let definition = create_definition();
        let factory = PrototypeScopeFactory;
        let mut scope = factory.create_scope();

        let instance = ComponentInstancePtr::new(0) as ComponentInstanceAnyPtr;
        scope.store_instance(&definition, instance.clone());

        assert!(scope.instance(&definition).is_none());
    }
}
