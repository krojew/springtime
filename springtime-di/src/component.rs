//! One of the basic blocks of dependency injection is a [Component]. Components are injectable
//! objects, which themselves can contain dependencies to other components.
//!
//! ## Registering concrete components
//!
//! Any type which wants to be managed by the DI system, needs to implement `Component`. For
//! convenience, the trait can be automatically derived with all infrastructure if the `derive`
//! feature is enabled:
//!
//! ```
//! use springtime_di::component::Component;
//! use springtime_di::instance_provider::ComponentInstancePtr;
//! use springtime_di::{Component, component_alias};
//!
//! trait TestTrait {}
//!
//! #[derive(Component)]
//! struct TestDependency;
//!
//! #[component_alias]
//! impl TestTrait for TestDependency {}
//!
//! #[derive(Component)]
//! #[component(names = ["dep2"])]
//! struct TestComponent {
//!     // concrete type dependency
//!     dependency_1: ComponentInstancePtr<TestDependency>,
//!     // primary dyn Trait dependency - note Send + Sync when using the "threadsafe" feature
//!     dependency_2: ComponentInstancePtr<dyn TestTrait + Send + Sync>,
//!     // optional dependency - don't fail, when not present
//!     optional_dependency: Option<ComponentInstancePtr<TestDependency>>,
//!     // all registered dependencies of given type
//!     all_deps: Vec<ComponentInstancePtr<dyn TestTrait + Sync + Send>>,
//!     #[component(default)]
//!     default: i8,
//!     #[component(default = "dummy_expr")]
//!     default_expr: i8,
//! }
//!
//! fn dummy_expr() -> i8 {
//!     -1
//! }
//! ```
//!
//! ### Supported `#[component]` struct configuration
//!
//! * `names = ["name"]` - use given name list as the component names, instead of the auto-generated
//! one
//!
//! ### Supported `#[component]` field configuration
//!
//! * `default` - use `Default::default()` initialization
//! * `default = "expr"` - call `expr()` for initialization
//!
//! ## Registering component aliases
//!
//! Component aliases are different types, which can refer to a concrete component type. Usually
//! they are simply `dyn Traits`, which makes it possible to inject an abstract `dyn Trait` type
//! instead of a concrete component type.
//!
//! To automatically register a component alias, use the `#[component_alias]` attribute on a trait
//! implementation:
//!
//! ```
//! use springtime_di::{Component, component_alias};
//!
//! #[derive(Component)]
//! struct SomeComponent;
//!
//! trait SomeTrait {
//! }
//!
//! #[component_alias]
//! impl SomeTrait for SomeComponent {
//! }
//! ```
//!
//! The above example shows how it's possible to inject both `ComponentInstancePtr<SomeComponent>`
//! and `ComponentInstancePtr<dyn SomeTrait>`.
//!
//! ### Supported `#[component_alias]` arguments
//!
//! * `primary` - mark the concrete component, for which we're implementing the trait, as selected
//! (primary) when requesting a single instance of `ComponentInstancePtr<dyn Trait>` and multiple
//! components are available

use crate::error::ComponentInstanceProviderError;
use crate::instance_provider::{
    ComponentInstanceAnyPtr, ComponentInstanceProvider, ComponentInstancePtr,
};

/// Base trait for components for dependency injection.
///
/// Components might depend on other components, which forms the basis for dependency injection. To
/// make the system work, your component instances must be wrapped in a [ComponentInstancePtr].
/// Please see the module-level documentation for more information.
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
