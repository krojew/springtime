//! The core functionality of creating and managing [Component](crate::component::Component)
//! instances.

use crate::component::Injectable;
use itertools::Itertools;
#[cfg(test)]
use mockall::automock;
use std::any::{Any, TypeId};
#[cfg(not(feature = "threadsafe"))]
use std::rc::Rc;
#[cfg(feature = "threadsafe")]
use std::sync::Arc;
use thiserror::Error;

/// Errors related to creating and managing components.
#[derive(Error, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum ComponentInstanceProviderError {
    #[error("Cannot find a primary instance for component '{0:?}' - either none or multiple exists without a primary marker.")]
    NoPrimaryInstance(TypeId),
    #[error("Tried to downcast component to incompatible type: {0:?}")]
    IncompatibleComponent(TypeId),
    #[error("Cannot find named component: {0}")]
    NoNamedInstance(String),
    #[error("Unrecognized scope: {0}")]
    UnrecognizedScope(String),
    #[error("Detected dependency cycle for: {0:?}")]
    DependencyCycle(TypeId),
}

#[cfg(not(feature = "threadsafe"))]
pub type ComponentInstancePtr<T> = Rc<T>;
#[cfg(feature = "threadsafe")]
pub type ComponentInstancePtr<T> = Arc<T>;

#[cfg(not(feature = "threadsafe"))]
pub type ComponentInstanceAnyPtr = ComponentInstancePtr<dyn Any + 'static>;
#[cfg(feature = "threadsafe")]
pub type ComponentInstanceAnyPtr = ComponentInstancePtr<dyn Any + Send + Sync + 'static>;

/// (Usually generated) cast function which consumes given type-erased instance pointer and casts it
/// to the desired [`ComponentInstancePtr<T>`]. The result is then returned as type-erased `Box` which
/// is then converted back to [`ComponentInstancePtr<T>`]. Such shenanigans are needed to be able to
/// convert between two `dyn Traits`.
pub type CastFunction =
    fn(instance: ComponentInstanceAnyPtr) -> Result<Box<dyn Any>, ComponentInstanceAnyPtr>;

/// Generic provider for component instances.
#[cfg_attr(test, automock)]
pub trait ComponentInstanceProvider {
    /// Tries to return a primary instance of a given component. A primary component is either the
    /// only one registered or one marked as primary.
    fn primary_instance(
        &mut self,
        type_id: TypeId,
    ) -> Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError>;

    /// Tries to instantiate and return all registered components for given type, stopping on first
    /// error. Be aware this might be an expensive operation if the number of registered components
    /// is high.
    fn instances(
        &mut self,
        type_id: TypeId,
    ) -> Result<Vec<(ComponentInstanceAnyPtr, CastFunction)>, ComponentInstanceProviderError>;

    /// Tries to return an instance with the given name.
    fn instance_by_name(
        &mut self,
        name: &str,
    ) -> Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError>;
}

/// Helper trait for [ComponentInstanceProvider] providing strongly-typed access.
pub trait TypedComponentInstanceProvider {
    /// Typesafe version of [ComponentInstanceProvider::primary_instance].
    fn primary_instance_typed<T: Injectable + ?Sized>(
        &mut self,
    ) -> Result<ComponentInstancePtr<T>, ComponentInstanceProviderError>;

    /// Tries to get an instance like [TypedComponentInstanceProvider::primary_instance_typed] does,
    /// but returns `None` on missing instance.
    fn primary_instance_option<T: Injectable + ?Sized>(
        &mut self,
    ) -> Result<Option<ComponentInstancePtr<T>>, ComponentInstanceProviderError>;

    /// Typesafe version of [ComponentInstanceProvider::instances].
    fn instances_typed<T: Injectable + ?Sized>(
        &mut self,
    ) -> Result<Vec<ComponentInstancePtr<T>>, ComponentInstanceProviderError>;

    /// Typesafe version of [ComponentInstanceProvider::instance_by_name].
    fn instance_by_name_typed<T: Injectable + ?Sized>(
        &mut self,
        name: &str,
    ) -> Result<ComponentInstancePtr<T>, ComponentInstanceProviderError>;

    /// Tries to get an instance like [TypedComponentInstanceProvider::instance_by_name_typed] does,
    /// but returns `None` on missing instance.
    fn instance_by_name_option<T: Injectable + ?Sized>(
        &mut self,
        name: &str,
    ) -> Result<Option<ComponentInstancePtr<T>>, ComponentInstanceProviderError>;
}

impl<CIP: ComponentInstanceProvider + ?Sized> TypedComponentInstanceProvider for CIP {
    fn primary_instance_typed<T: Injectable + ?Sized>(
        &mut self,
    ) -> Result<ComponentInstancePtr<T>, ComponentInstanceProviderError> {
        let type_id = TypeId::of::<T>();
        self.primary_instance(type_id)
            .and_then(move |(p, cast)| cast_instance(p, cast, type_id))
    }

    fn primary_instance_option<T: Injectable + ?Sized>(
        &mut self,
    ) -> Result<Option<ComponentInstancePtr<T>>, ComponentInstanceProviderError> {
        match self.primary_instance_typed::<T>() {
            Ok(ptr) => Ok(Some(ptr)),
            Err(ComponentInstanceProviderError::NoPrimaryInstance(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn instances_typed<T: Injectable + ?Sized>(
        &mut self,
    ) -> Result<Vec<ComponentInstancePtr<T>>, ComponentInstanceProviderError> {
        let type_id = TypeId::of::<T>();
        self.instances(type_id).and_then(|instances| {
            instances
                .into_iter()
                .map(move |(p, cast)| cast_instance(p, cast, type_id))
                .try_collect()
        })
    }

    fn instance_by_name_typed<T: Injectable + ?Sized>(
        &mut self,
        name: &str,
    ) -> Result<ComponentInstancePtr<T>, ComponentInstanceProviderError> {
        self.instance_by_name(name)
            .and_then(move |(p, cast)| cast_instance(p, cast, TypeId::of::<T>()))
    }

    fn instance_by_name_option<T: Injectable + ?Sized>(
        &mut self,
        name: &str,
    ) -> Result<Option<ComponentInstancePtr<T>>, ComponentInstanceProviderError> {
        match self.instance_by_name_typed::<T>(name) {
            Ok(ptr) => Ok(Some(ptr)),
            Err(ComponentInstanceProviderError::NoPrimaryInstance(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn cast_instance<T: Injectable + ?Sized>(
    instance: ComponentInstanceAnyPtr,
    cast: CastFunction,
    type_id: TypeId,
) -> Result<ComponentInstancePtr<T>, ComponentInstanceProviderError> {
    debug_assert_eq!(type_id, TypeId::of::<T>());
    cast(instance)
        .map_err(|_| ComponentInstanceProviderError::IncompatibleComponent(type_id))
        .and_then(|p| {
            p.downcast::<ComponentInstancePtr<T>>()
                .map(|p| (*p).clone())
                .map_err(|_| ComponentInstanceProviderError::IncompatibleComponent(type_id))
        })
}

#[cfg(test)]
//noinspection DuplicatedCode
mod tests {
    use crate::component::Injectable;
    use crate::instance_provider::{
        CastFunction, ComponentInstanceAnyPtr, ComponentInstancePtr, MockComponentInstanceProvider,
        TypedComponentInstanceProvider,
    };
    use mockall::predicate::*;
    use std::any::{Any, TypeId};

    struct TestComponent;

    impl Injectable for TestComponent {}

    fn test_cast(
        instance: ComponentInstanceAnyPtr,
    ) -> Result<Box<dyn Any>, ComponentInstanceAnyPtr> {
        instance
            .downcast::<TestComponent>()
            .map(|p| Box::new(p) as Box<dyn Any>)
    }

    #[test]
    fn should_provide_primary_instance_typed() {
        let mut instance_provider = MockComponentInstanceProvider::new();
        instance_provider
            .expect_primary_instance()
            .with(eq(TypeId::of::<TestComponent>()))
            .times(1)
            .return_const(Ok((
                ComponentInstancePtr::new(TestComponent) as ComponentInstanceAnyPtr,
                test_cast as CastFunction,
            )));

        assert!(instance_provider
            .primary_instance_typed::<TestComponent>()
            .is_ok());
    }

    #[test]
    fn should_provide_primary_instance_option() {
        let mut instance_provider = MockComponentInstanceProvider::new();
        instance_provider
            .expect_primary_instance()
            .with(eq(TypeId::of::<TestComponent>()))
            .times(1)
            .return_const(Ok((
                ComponentInstancePtr::new(TestComponent) as ComponentInstanceAnyPtr,
                test_cast as CastFunction,
            )));

        assert!(instance_provider
            .primary_instance_option::<TestComponent>()
            .unwrap()
            .is_some());
    }

    #[test]
    fn should_provide_instances_typed() {
        let mut instance_provider = MockComponentInstanceProvider::new();
        instance_provider
            .expect_instances()
            .with(eq(TypeId::of::<TestComponent>()))
            .times(1)
            .return_const(Ok(vec![(
                ComponentInstancePtr::new(TestComponent) as ComponentInstanceAnyPtr,
                test_cast as CastFunction,
            )]));

        assert!(!instance_provider
            .instances_typed::<TestComponent>()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn should_provide_instance_by_name_typed() {
        let name = "name";

        let mut instance_provider = MockComponentInstanceProvider::new();
        instance_provider
            .expect_instance_by_name()
            .with(eq(name))
            .times(1)
            .return_const(Ok((
                ComponentInstancePtr::new(TestComponent) as ComponentInstanceAnyPtr,
                test_cast as CastFunction,
            )));

        assert!(instance_provider
            .instance_by_name_typed::<TestComponent>(name)
            .is_ok());
    }

    #[test]
    fn should_provide_instance_by_name_option() {
        let name = "name";

        let mut instance_provider = MockComponentInstanceProvider::new();
        instance_provider
            .expect_instance_by_name()
            .with(eq(name))
            .times(1)
            .return_const(Ok((
                ComponentInstancePtr::new(TestComponent) as ComponentInstanceAnyPtr,
                test_cast as CastFunction,
            )));

        assert!(instance_provider
            .instance_by_name_option::<TestComponent>(name)
            .unwrap()
            .is_some());
    }
}
