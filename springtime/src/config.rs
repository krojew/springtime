//! Framework configuration is based on injecting an [ApplicationConfigProvider], which can later
//! be used to retrieve [ApplicationConfig]. [Application](crate::application::Application) uses
//! this config to configure itself, but it can also be injected into any other component.
//!
//! By default, the config is created with opinionated default values, which can then be overwritten
//! by environment variables prefixed with `SPRINGTIME_` or `springtime.json` file.

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use springtime_di::component_registry::conditional::unregistered_component;
#[cfg(feature = "async")]
use springtime_di::future::{BoxFuture, FutureExt};
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, injectable, Component};
use std::error::Error;

const CONFIG_ENV_PREFIX: &str = "SPRINGTIME";

/// Name of the default config file.
pub const CONFIG_FILE: &str = "springtime.json";

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

/// Framework configuration which can be provided by an [ApplicationConfigProvider].
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct ApplicationConfig {
    /// Should a default tracing logger be installed in the scope of the application.
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
    fn init_from_environment() -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::with_name(CONFIG_FILE).required(false))
            .add_source(Environment::with_prefix(CONFIG_ENV_PREFIX))
            .build()
            .and_then(|config| config.try_deserialize::<OptionalApplicationConfig>())
            .map(|config| config.into())
    }
}

/// Provider for [ApplicationConfig]. The primary instance of the provider will be used to retrieve
/// application configuration.
#[injectable]
pub trait ApplicationConfigProvider {
    #[cfg(feature = "async")]
    fn config(&self) -> BoxFuture<'_, Result<&ApplicationConfig, ErrorPtr>>;

    #[cfg(not(feature = "async"))]
    fn config(&self) -> Result<&ApplicationConfig, ErrorPtr>;
}

#[derive(Component)]
#[cfg_attr(feature = "threadsafe", component(priority = -128, condition = "unregistered_component::<dyn ApplicationConfigProvider + Send + Sync>", constructor = "DefaultApplicationConfigProvider::new"))]
#[cfg_attr(not(feature = "threadsafe"), component(priority = -128, condition = "unregistered_component::<dyn ApplicationConfigProvider>", constructor = "DefaultApplicationConfigProvider::new"))]
struct DefaultApplicationConfigProvider {
    // cached init result
    #[component(ignore)]
    config: Result<ApplicationConfig, ErrorPtr>,
}

impl DefaultApplicationConfigProvider {
    #[cfg(feature = "async")]
    fn new() -> BoxFuture<'static, Result<Self, ErrorPtr>> {
        async {
            Ok(Self {
                config: ApplicationConfig::init_from_environment().map_err(convert_error),
            })
        }
        .boxed()
    }

    #[cfg(not(feature = "async"))]
    fn new() -> Result<Self, ErrorPtr> {
        Ok(Self {
            config: ApplicationConfig::init_from_environment().map_err(convert_error),
        })
    }

    fn map_config(&self) -> Result<&ApplicationConfig, ErrorPtr> {
        match &self.config {
            Ok(config) => Ok(config),
            Err(error) => Err(error.clone()),
        }
    }
}

#[component_alias]
impl ApplicationConfigProvider for DefaultApplicationConfigProvider {
    #[cfg(feature = "async")]
    fn config(&self) -> BoxFuture<'_, Result<&ApplicationConfig, ErrorPtr>> {
        async { self.map_config() }.boxed()
    }

    #[cfg(not(feature = "async"))]
    fn config(&self) -> Result<&ApplicationConfig, ErrorPtr> {
        self.map_config()
    }
}

#[derive(Deserialize)]
struct OptionalApplicationConfig {
    install_tracing_logger: Option<bool>,
}
