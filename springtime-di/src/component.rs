use crate::error::ComponentInstanceProviderError;
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
}

/// Helper trait for [ComponentInstanceProvider] providing strongly-typed access.
pub trait TypedComponentInstanceProvider {
    /// Typesafe version of [ComponentInstanceProvider::primary_instance].
    fn primary_instance_typed<T: ComponentDowncast + ?Sized + 'static>(
        &self,
    ) -> Result<ComponentInstancePtr<T>, ComponentInstanceProviderError>;

    /// Tries to get an instance like [TypedComponentInstanceProvider::primary_instance_typed] does,
    /// but returns None on missing instance.
    fn primary_instance_option<T: ComponentDowncast + ?Sized + 'static>(
        &self,
    ) -> Result<Option<ComponentInstancePtr<T>>, ComponentInstanceProviderError>;
}

impl<CIP: ComponentInstanceProvider + ?Sized> TypedComponentInstanceProvider for CIP {
    fn primary_instance_typed<T: ComponentDowncast + ?Sized + 'static>(
        &self,
    ) -> Result<ComponentInstancePtr<T>, ComponentInstanceProviderError> {
        let type_id = TypeId::of::<T>();
        self.primary_instance(type_id).and_then(|p| {
            T::downcast(p).map_err(|_| ComponentInstanceProviderError::NoPrimaryInstance(type_id))
        })
    }

    fn primary_instance_option<T: ComponentDowncast + ?Sized + 'static>(
        &self,
    ) -> Result<Option<ComponentInstancePtr<T>>, ComponentInstanceProviderError> {
        match self.primary_instance_typed::<T>() {
            Ok(ptr) => Ok(Some(ptr)),
            Err(ComponentInstanceProviderError::NoPrimaryInstance(_)) => Ok(None),
        }
    }
}

/// Base trait for components for dependency injection.
///
/// Components might depend on other components, which forms the basis for dependency injection. To
/// make the system work, your component instances must be wrapped in a [ComponentInstancePtr].
///
/// ## Registering concrete components
///
/// Any type which wants to be managed by the DI system, needs to implement `Component`. For
/// convenience, the trait can be automatically derived with all infrastructure if the `derive`
/// feature is enabled:
///
/// ```
/// use springtime_di::component::{ComponentInstancePtr, Component};
/// use springtime_di::{Component, component_alias};
///
/// trait TestTrait {}
///
/// #[derive(Component)]
/// struct TestDependency;
///
/// #[component_alias]
/// impl TestTrait for TestDependency {}
///
/// #[derive(Component)]
/// #[component(names = ["dep2"])]
/// struct TestComponent {
///     // concrete type dependency
///     _dependency_1: ComponentInstancePtr<TestDependency>,
///     // primary dyn Trait dependency - note Send + Sync when using the "threadsafe" feature
///     _dependency_2: ComponentInstancePtr<dyn TestTrait + Send + Sync>,
///     // optional dependency - don't fail, when not present
///     _optional_dependency: Option<ComponentInstancePtr<TestDependency>>,
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
/// ### Supported `#[component]` struct configuration
///
/// * `names = ["name"]` - use given name list as the component names, instead of the auto-generated
/// one
///
/// ### Supported `#[component]` field configuration
///
/// * `default` - use `Default::default()` initialization
/// * `default = "expr"` - call `expr()` for initialization
///
/// ## Registering component aliases
///
/// Component aliases are different types, which can refer to a concrete component type. Usually
/// they are simply `dyn Traits`, which makes it possible to inject an abstract `dyn Trait` type
/// instead of a concrete component type.
///
/// To automatically register a component alias, use the `#[component_alias]` attribute on a trait
/// implementation:
///
/// ```
/// use springtime_di::{Component, component_alias};
///
/// #[derive(Component)]
/// struct SomeComponent;
///
/// trait SomeTrait {
/// }
///
/// #[component_alias]
/// impl SomeTrait for SomeComponent {
/// }
/// ```
///
/// The above example shows how it's possible to inject both `ComponentInstancePtr<SomeComponent>`
/// and `ComponentInstancePtr<dyn SomeTrait>`.
///
/// ### Supported `#[component_alias]` arguments
///
/// * `primary` - mark the concrete component, for which we're implementing the trait, as selected
/// (primary) when requesting a single instance of `ComponentInstancePtr<dyn Trait>` and multiple
/// components are available
pub trait Component: ComponentDowncast {
    /// Creates an instance of this component using dependencies from given [ComponentInstanceProvider].
    fn create(
        instance_provider: &dyn ComponentInstanceProvider,
    ) -> Result<Self, ComponentInstanceProviderError>
    where
        Self: Sized;
}

/// Helper trait for traits implemented by components, thus allowing injection of components based
/// on `dyn Trait` types. Typically automatically derived when using the `#[component_alias]`
/// attribute.
pub trait ComponentDowncast: Injectable {
    fn downcast(
        source: ComponentInstanceAnyPtr,
    ) -> Result<ComponentInstancePtr<Self>, ComponentInstanceAnyPtr>;
}

/// Marker trait for injectable types - components and aliases.
pub trait Injectable {}
