use crate::Error;
#[cfg(not(feature = "threadsafe"))]
use std::rc::Rc;
#[cfg(feature = "threadsafe")]
use std::sync::Arc;

#[cfg(not(feature = "threadsafe"))]
pub type ComponentInstancePtr<T> = Rc<T>;
#[cfg(feature = "threadsafe")]
pub type ComponentInstancePtr<T> = Arc<T>;

pub trait ComponentInstanceProvider {
    /// Tries to return a primary instance of a given component. A primary component is either the
    /// only one registered or one marked as primary.
    fn primary_instance<T: Component + 'static>(&self) -> Result<ComponentInstancePtr<T>, Error>;
}

pub trait Component {
    /// Creates an instance of this component using dependencies from given [ComponentInstanceProvider].
    fn create<CIP: ComponentInstanceProvider>(
        instance_provider: &CIP,
    ) -> Result<ComponentInstancePtr<Self>, Error>;
}
