use crate::error::ComponentInstanceProviderError;
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
    fn primary_instance<T: Component + 'static>(
        &self,
    ) -> Result<ComponentInstancePtr<T>, ComponentInstanceProviderError>;
}

/// Base trait for components for dependency injection.
///
/// Components might depend on other components, which forms the basis for dependency injection. To
/// make the system work, your component instances must be wrapped in a [ComponentInstancePtr].
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
/// #[component(name = "dep2", primary)]
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
/// ### Supported `#[component]` struct configuration:
///
/// * `name = "name"` - use given name as the component name, instead of an auto-generated one
/// * `primary` - mark the given component as primary within other components of given type
///
/// ### Supported `#[component]` field configuration:
///
/// * `default` - use `Default::default()` initialization
/// * `default = "expr"` - call `expr()` for initialization
pub trait Component {
    /// Creates an instance of this component using dependencies from given [ComponentInstanceProvider].
    fn create<CIP: ComponentInstanceProvider>(
        instance_provider: &CIP,
    ) -> Result<Self, ComponentInstanceProviderError>
    where
        Self: Sized;
}
