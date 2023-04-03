use std::any::TypeId;
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
}

/// Error related to component registries.
#[derive(Error, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum ComponentDefinitionRegistryError {
    #[error("Attempted to register a duplicated component with name: {0}")]
    DuplicateComponentName(String),
    #[error("Attempted to re-register a concrete component type: {0}")]
    DuplicateComponentType(String),
    #[error("Missing base component of type {target_type} for alias: {alias_type}")]
    MissingBaseComponent {
        alias_type: String,
        target_type: String,
    },
    #[error(
        "Registering a duplicate primary component of type {target_type} for alias: {alias_type}"
    )]
    DuplicatePrimaryComponent {
        alias_type: String,
        target_type: String,
    },
    #[error("Cannot register alias {alias_type} named {name} for multiple targets: {target_type}")]
    CannotRegisterNamedAliasForAmbiguousTarget {
        alias_type: String,
        target_type: String,
        name: String,
    },
}
