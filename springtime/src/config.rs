//! Framework configuration.

use config::{Config, Environment};
use serde::Deserialize;
use springtime_di::component_registry::conditional::unregistered_component;
#[cfg(feature = "async")]
use springtime_di::future::{BoxFuture, FutureExt};
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::Component;
use std::error::Error;

const CONFIG_ENV_PREFIX: &str = "SPRINGTIME";

#[cfg(feature = "threadsafe")]
fn convert_error<E: Error + Send + Sync + 'static>(error: E) -> ErrorPtr {
    use std::sync::Arc;
    Arc::new(error) as ErrorPtr
}

#[cfg(not(feature = "threadsafe"))]
fn convert_error<E: Error + 'static>(error: E) -> ErrorPtr {
    use std::rc::Rc;
    Rc::new(error) as ErrorPtr
}

/// Framework configuration which can be provided by injection.
#[non_exhaustive]
#[derive(Clone, Component, Debug)]
#[component(constructor = "ApplicationConfig::init_from_environment", priority = -128, condition = "unregistered_component::<ApplicationConfig>")]
pub struct ApplicationConfig {
    /// Should a default global tracing logger be installed.
    #[component(ignore)]
    pub install_tracing_logger: bool,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            install_tracing_logger: true,
        }
    }
}

impl From<OptionalApplicationConfig> for ApplicationConfig {
    fn from(value: OptionalApplicationConfig) -> Self {
        let default = Self::default();
        Self {
            install_tracing_logger: value
                .install_tracing_logger
                .unwrap_or(default.install_tracing_logger),
        }
    }
}

impl ApplicationConfig {
    #[cfg(feature = "async")]
    fn init_from_environment() -> BoxFuture<'static, Result<Self, ErrorPtr>> {
        async { Self::try_init() }.boxed()
    }

    #[cfg(not(feature = "async"))]
    fn init_from_environment() -> Result<Self, ErrorPtr> {
        Self::try_init()
    }

    fn try_init() -> Result<Self, ErrorPtr> {
        Config::builder()
            .add_source(Environment::with_prefix(CONFIG_ENV_PREFIX))
            .build()
            .and_then(|config| config.try_deserialize::<OptionalApplicationConfig>())
            .map(|config| config.into())
            .map_err(convert_error)
    }
}

#[derive(Deserialize)]
struct OptionalApplicationConfig {
    install_tracing_logger: Option<bool>,
}
