//! Framework configuration is based on injecting an [WebConfigProvider], which can later be used to
//! retrieve [WebConfig].
//!
//! By default, the config is created with opinionated default values, which can then be overwritten
//! by values from `springtime.json` file under the `web` key.

use config::{Config, File};
use fxhash::FxHashMap;
use serde::Deserialize;
use springtime::config::CONFIG_FILE;
use springtime::future::{BoxFuture, FutureExt};
use springtime_di::component_registry::conditional::unregistered_component;
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, injectable, Component};
use std::sync::Arc;

/// Name of the default server present in the default [WebConfig].
pub const DEFAULT_SERVER_NAME: &str = "default";

/// Server configuration.
#[non_exhaustive]
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
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
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct WebConfig {
    /// Map from server name to their config. Typically, only one server with one address will be
    /// present (see: [DEFAULT_SERVER_NAME], but in case multiple servers are desired, they should
    /// be specified here.
    pub servers: FxHashMap<String, ServerConfig>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            servers: [(DEFAULT_SERVER_NAME.to_string(), Default::default())]
                .into_iter()
                .collect(),
        }
    }
}

impl WebConfig {
    fn init_from_config() -> Result<Self, ErrorPtr> {
        Config::builder()
            .add_source(File::with_name(CONFIG_FILE).required(false))
            .build()
            .and_then(|config| config.try_deserialize::<WebConfigWrapper>())
            .map(|config| config.web)
            .map_err(|error| Arc::new(error) as ErrorPtr)
    }
}

/// Provider for [WebConfig]. The primary instance of the provider will be used to retrieve web
/// configuration.
#[injectable]
pub trait WebConfigProvider {
    /// Provide current config.
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

#[derive(Deserialize, Default)]
#[serde(default)]
struct WebConfigWrapper {
    web: WebConfig,
}
