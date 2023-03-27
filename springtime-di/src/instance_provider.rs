use crate::component::ComponentDowncast;
use crate::error::ComponentInstanceProviderError;
use itertools::Itertools;
use std::any::{Any, TypeId};
#[cfg(not(feature = "threadsafe"))]
use std::rc::Rc;
#[cfg(feature = "threadsafe")]
use std::sync::Arc;

#[cfg(not(feature = "threadsafe"))]
pub type ComponentInstancePtr<T> = Rc<T>;
#[cfg(feature = "threadsafe")]
pub type ComponentInstancePtr<T> = Arc<T>;

#[cfg(not(feature = "threadsafe"))]
pub type ComponentInstanceAnyPtr = ComponentInstancePtr<dyn Any + 'static>;
#[cfg(feature = "threadsafe")]
pub type ComponentInstanceAnyPtr = ComponentInstancePtr<dyn Any + Send + Sync + 'static>;

/// Generic provider for component instances.
pub trait ComponentInstanceProvider {
    /// Tries to return a primary instance of a given component. A primary component is either the
    /// only one registered or one marked as primary.
    fn primary_instance(
        &self,
        type_id: TypeId,
    ) -> Result<ComponentInstanceAnyPtr, ComponentInstanceProviderError>;

    /// Tries to instantiate and return all registered components for given type, stopping on first
    /// error.
    fn instances(
        &self,
        type_id: TypeId,
    ) -> Result<Vec<ComponentInstanceAnyPtr>, ComponentInstanceProviderError>;
}

/// Helper trait for [ComponentInstanceProvider] providing strongly-typed access.
pub trait TypedComponentInstanceProvider {
    /// Typesafe version of [ComponentInstanceProvider::primary_instance].
    fn primary_instance_typed<T: ComponentDowncast + ?Sized + 'static>(
        &self,
    ) -> Result<ComponentInstancePtr<T>, ComponentInstanceProviderError>;

    /// Tries to get an instance like [TypedComponentInstanceProvider::primary_instance_typed] does,
    /// but returns `None` on missing instance.
    fn primary_instance_option<T: ComponentDowncast + ?Sized + 'static>(
        &self,
    ) -> Result<Option<ComponentInstancePtr<T>>, ComponentInstanceProviderError>;

    /// Typesafe version of [ComponentInstanceProvider::instances].
    fn instances_typed<T: ComponentDowncast + ?Sized + 'static>(
        &self,
    ) -> Result<Vec<ComponentInstancePtr<T>>, ComponentInstanceProviderError>;
}

impl<CIP: ComponentInstanceProvider + ?Sized> TypedComponentInstanceProvider for CIP {
    fn primary_instance_typed<T: ComponentDowncast + ?Sized + 'static>(
        &self,
    ) -> Result<ComponentInstancePtr<T>, ComponentInstanceProviderError> {
        let type_id = TypeId::of::<T>();
        self.primary_instance(type_id).and_then(|p| {
            T::downcast(p)
                .map_err(|_| ComponentInstanceProviderError::IncompatibleComponent(type_id))
        })
    }

    fn primary_instance_option<T: ComponentDowncast + ?Sized + 'static>(
        &self,
    ) -> Result<Option<ComponentInstancePtr<T>>, ComponentInstanceProviderError> {
        match self.primary_instance_typed::<T>() {
            Ok(ptr) => Ok(Some(ptr)),
            Err(ComponentInstanceProviderError::NoPrimaryInstance(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn instances_typed<T: ComponentDowncast + ?Sized + 'static>(
        &self,
    ) -> Result<Vec<ComponentInstancePtr<T>>, ComponentInstanceProviderError> {
        let type_id = TypeId::of::<T>();
        self.instances(type_id).and_then(|instances| {
            instances
                .into_iter()
                .map(|p| {
                    T::downcast(p)
                        .map_err(|_| ComponentInstanceProviderError::IncompatibleComponent(type_id))
                })
                .try_collect()
        })
    }
}
