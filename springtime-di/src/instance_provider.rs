//! The core functionality of creating and managing [Component](crate::component::Component)
//! instances.

use crate::component::Injectable;
#[cfg(feature = "async")]
use futures::future::BoxFuture;
#[cfg(feature = "async")]
use futures::FutureExt;
use itertools::Itertools;
#[cfg(test)]
use mockall::automock;
use std::any::{Any, TypeId};
use std::error::Error;
#[cfg(not(feature = "threadsafe"))]
use std::rc::Rc;
#[cfg(feature = "threadsafe")]
use std::sync::Arc;
use thiserror::Error;

#[cfg(not(feature = "threadsafe"))]
pub type ErrorPtr = Rc<dyn Error>;
#[cfg(feature = "threadsafe")]
pub type ErrorPtr = Arc<dyn Error + Send + Sync>;

/// Errors related to creating and managing components.
#[derive(Error, Debug, Clone)]
pub enum ComponentInstanceProviderError {
    /// Primary instance of a given component is not specified, if many components exist for a given
    /// type, or not component registered at all.
    #[error("Cannot find a primary instance for component '{0:?}' - either none or multiple exists without a primary marker.")]
    NoPrimaryInstance(TypeId),
    /// Tired to case one type to another, incompatible one.
    #[error("Tried to downcast component to incompatible type: {0:?}")]
    IncompatibleComponent(TypeId),
    /// Cannot find component with given name.
    #[error("Cannot find named component: {0}")]
    NoNamedInstance(String),
    /// Component registered for unknown scope - possibly missing associated
    /// [ScopeFactory](crate::scope::ScopeFactory).
    #[error("Unrecognized scope: {0}")]
    UnrecognizedScope(String),
    #[error("Detected dependency cycle for: {0:?}")]
    /// Found a cycle when creating given type.
    DependencyCycle(TypeId),
    /// Custom constructor returned an error.
    #[error("Error in component constructor: {0}")]
    ConstructorError(#[source] ErrorPtr),
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
#[cfg(feature = "async")]
#[cfg_attr(test, automock)]
pub trait ComponentInstanceProvider {
    /// Tries to return a primary instance of a given component. A primary component is either the
    /// only one registered or one marked as primary.
    fn primary_instance(
        &mut self,
        type_id: TypeId,
    ) -> BoxFuture<
        '_,
        Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError>,
    >;

    /// Tries to instantiate and return all registered components for given type, stopping on first
    /// error. Be aware this might be an expensive operation if the number of registered components
    /// is high.
    fn instances(
        &mut self,
        type_id: TypeId,
    ) -> BoxFuture<
        '_,
        Result<Vec<(ComponentInstanceAnyPtr, CastFunction)>, ComponentInstanceProviderError>,
    >;

    /// Tries to return an instance with the given name and type.
    fn instance_by_name(
        &mut self,
        name: &str,
        type_id: TypeId,
    ) -> BoxFuture<
        '_,
        Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError>,
    >;
}

#[cfg(not(feature = "async"))]
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

    /// Tries to return an instance with the given name and type.
    fn instance_by_name(
        &mut self,
        name: &str,
        type_id: TypeId,
    ) -> Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError>;
}

/// Helper trait for [ComponentInstanceProvider] providing strongly-typed access.
#[cfg(feature = "async")]
pub trait TypedComponentInstanceProvider {
    /// Typesafe version of [ComponentInstanceProvider::primary_instance].
    fn primary_instance_typed<T: Injectable + ?Sized>(
        &mut self,
    ) -> BoxFuture<'_, Result<ComponentInstancePtr<T>, ComponentInstanceProviderError>>;

    /// Tries to get an instance like [TypedComponentInstanceProvider::primary_instance_typed] does,
    /// but returns `None` on missing instance.
    fn primary_instance_option<T: Injectable + ?Sized>(
        &mut self,
    ) -> BoxFuture<'_, Result<Option<ComponentInstancePtr<T>>, ComponentInstanceProviderError>>;

    /// Typesafe version of [ComponentInstanceProvider::instances].
    fn instances_typed<T: Injectable + ?Sized>(
        &mut self,
    ) -> BoxFuture<'_, Result<Vec<ComponentInstancePtr<T>>, ComponentInstanceProviderError>>;

    /// Typesafe version of [ComponentInstanceProvider::instance_by_name].
    fn instance_by_name_typed<T: Injectable + ?Sized>(
        &mut self,
        name: &str,
    ) -> BoxFuture<'_, Result<ComponentInstancePtr<T>, ComponentInstanceProviderError>>;

    /// Tries to get an instance like [TypedComponentInstanceProvider::instance_by_name_typed] does,
    /// but returns `None` on missing instance.
    fn instance_by_name_option<T: Injectable + ?Sized>(
        &mut self,
        name: &str,
    ) -> BoxFuture<'_, Result<Option<ComponentInstancePtr<T>>, ComponentInstanceProviderError>>;
}

/// Helper trait for [ComponentInstanceProvider] providing strongly-typed access.
#[cfg(not(feature = "async"))]
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

//noinspection DuplicatedCode
#[cfg(feature = "async")]
impl<CIP: ComponentInstanceProvider + ?Sized + Sync + Send> TypedComponentInstanceProvider for CIP {
    fn primary_instance_typed<T: Injectable + ?Sized>(
        &mut self,
    ) -> BoxFuture<'_, Result<ComponentInstancePtr<T>, ComponentInstanceProviderError>> {
        async {
            let type_id = TypeId::of::<T>();
            self.primary_instance(type_id)
                .await
                .and_then(move |(p, cast)| cast_instance(p, cast, type_id))
        }
        .boxed()
    }

    fn primary_instance_option<T: Injectable + ?Sized>(
        &mut self,
    ) -> BoxFuture<'_, Result<Option<ComponentInstancePtr<T>>, ComponentInstanceProviderError>>
    {
        async {
            match self.primary_instance_typed::<T>().await {
                Ok(ptr) => Ok(Some(ptr)),
                Err(ComponentInstanceProviderError::NoPrimaryInstance(_)) => Ok(None),
                Err(error) => Err(error),
            }
        }
        .boxed()
    }

    fn instances_typed<T: Injectable + ?Sized>(
        &mut self,
    ) -> BoxFuture<'_, Result<Vec<ComponentInstancePtr<T>>, ComponentInstanceProviderError>> {
        async {
            let type_id = TypeId::of::<T>();
            self.instances(type_id).await.and_then(|instances| {
                instances
                    .into_iter()
                    .map(move |(p, cast)| cast_instance(p, cast, type_id))
                    .try_collect()
            })
        }
        .boxed()
    }

    fn instance_by_name_typed<T: Injectable + ?Sized>(
        &mut self,
        name: &str,
    ) -> BoxFuture<'_, Result<ComponentInstancePtr<T>, ComponentInstanceProviderError>> {
        let name = name.to_string();
        async move {
            let type_id = TypeId::of::<T>();
            self.instance_by_name(&name, type_id)
                .await
                .and_then(move |(p, cast)| cast_instance(p, cast, type_id))
        }
        .boxed()
    }

    fn instance_by_name_option<T: Injectable + ?Sized>(
        &mut self,
        name: &str,
    ) -> BoxFuture<'_, Result<Option<ComponentInstancePtr<T>>, ComponentInstanceProviderError>>
    {
        let name = name.to_string();
        async move {
            match self.instance_by_name_typed::<T>(&name).await {
                Ok(ptr) => Ok(Some(ptr)),
                Err(ComponentInstanceProviderError::NoPrimaryInstance(_)) => Ok(None),
                Err(error) => Err(error),
            }
        }
        .boxed()
    }
}

//noinspection DuplicatedCode
#[cfg(not(feature = "async"))]
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
        let type_id = TypeId::of::<T>();
        self.instance_by_name(name, type_id)
            .and_then(move |(p, cast)| cast_instance(p, cast, type_id))
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
    #[cfg(not(feature = "async"))]
    mod sync {
        use crate::component::Injectable;
        use crate::instance_provider::{
            CastFunction, ComponentInstanceAnyPtr, ComponentInstancePtr,
            MockComponentInstanceProvider, TypedComponentInstanceProvider,
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
                .with(eq(name), eq(TypeId::of::<TestComponent>()))
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
                .with(eq(name), eq(TypeId::of::<TestComponent>()))
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
}
