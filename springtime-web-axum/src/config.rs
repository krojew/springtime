//! Framework configuration is based on injecting an [WebConfigProvider], which can later be used to
//! retrieve [WebConfig].
//!
//! By default, the config is created with opinionated default values, which can then be overwritten
//! by values from `springtime.json` file under the `web` key.

use config::{Config, File};
use fxhash::FxHashMap;
use serde::Deserialize;
use springtime::config::CONFIG_FILE;
use springtime_di::component_registry::conditional::unregistered_component;
use springtime_di::future::{BoxFuture, FutureExt};
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, injectable, Component};
use std::sync::Arc;

/// Server configuration.
#[non_exhaustive]
#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    /// Address on which to listen.
    pub listen_address: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_address: "0.0.0.0:80".to_string(),
        }
    }
}

/// Framework configuration which can be provided by an [WebConfigProvider].
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct WebConfig {
    /// Map from server name to their config. Typically, only one server with one address will be
    /// present, but in case multiple servers are desired, they should be specified here.
    pub servers: FxHashMap<String, ServerConfig>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            servers: [("default".to_string(), Default::default())]
                .into_iter()
                .collect(),
        }
    }
}

impl From<OptionalWebConfig> for WebConfig {
    fn from(value: OptionalWebConfig) -> Self {
        let default = Self::default();
        Self {
            servers: value.servers.unwrap_or(default.servers),
        }
    }
}

impl WebConfig {
    fn init_from_config() -> Result<Self, ErrorPtr> {
        Config::builder()
            .add_source(File::with_name(CONFIG_FILE).required(false))
            .build()
            .and_then(|config| config.try_deserialize::<OptionalWebConfigWrapper>())
            .map(|config| config.web.map(|config| config.into()).unwrap_or_default())
            .map_err(|error| Arc::new(error) as ErrorPtr)
    }
}

/// Provider for [WebConfig]. The primary instance of the provider will be used to retrieve web
/// configuration.
#[injectable]
pub trait WebConfigProvider {
    fn config(&self) -> BoxFuture<'_, Result<&WebConfig, ErrorPtr>>;
}

#[derive(Component)]
#[component(priority = -128, condition = "unregistered_component::<dyn WebConfigProvider + Send + Sync>", constructor = "DefaultWebConfigProvider::new")]
struct DefaultWebConfigProvider {
    // cached init result
    #[component(ignore)]
    config: Result<WebConfig, ErrorPtr>,
}

#[component_alias]
impl WebConfigProvider for DefaultWebConfigProvider {
    fn config(&self) -> BoxFuture<'_, Result<&WebConfig, ErrorPtr>> {
        async {
            match &self.config {
                Ok(config) => Ok(config),
                Err(error) => Err(error.clone()),
            }
        }
        .boxed()
    }
}

impl DefaultWebConfigProvider {
    fn new() -> BoxFuture<'static, Result<Self, ErrorPtr>> {
        async {
            Ok(Self {
                config: WebConfig::init_from_config(),
            })
        }
        .boxed()
    }
}

#[derive(Deserialize)]
struct OptionalWebConfig {
    servers: Option<FxHashMap<String, ServerConfig>>,
}

#[derive(Deserialize)]
struct OptionalWebConfigWrapper {
    web: Option<OptionalWebConfig>,
}
