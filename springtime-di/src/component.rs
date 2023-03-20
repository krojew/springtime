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

/// Base trait for components for dependency injection.
///
/// Any type which wants to be managed by the DI system, needs to implement `Component`. For
/// convenience, the trait can be automatically derived with all infrastructure if the `derive`
/// feature is enabled:
///
/// ```
/// use springtime_di::component::{ComponentInstancePtr, Component};
/// use springtime_di_derive::Component;
///
/// #[derive(Component)]
/// struct TestDependency;
///
/// #[derive(Component)]
/// struct TestComponent {
///     _dependency: ComponentInstancePtr<TestDependency>,
///     #[component(default)]
///     _default: i8,
///     #[component(default = "dummy_expr")]
///     _default_expr: i8,
/// }
///
/// fn dummy_expr() -> i8 {
///     -1
/// }
/// ```
///
/// ### Supported `#[component]` configuration:
///
/// * `default` - use `Default::default()` initialization
/// * `default = "expr"` - call `expr()` for initialization
pub trait Component {
    /// Creates an instance of this component using dependencies from given [ComponentInstanceProvider].
    fn create<CIP: ComponentInstanceProvider>(instance_provider: &CIP) -> Result<Self, Error>
    where
        Self: Sized;
}
