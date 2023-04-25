//! Migration configuration is based on injecting an [MigrationConfigProvider], which can later be
//! used to retrieve [MigrationConfig].
//!
//! By default, the config is created with opinionated default values, which can then be overwritten
//! by values from `springtime.json` file under the `migration` key.

use config::{Config, File};
use serde::Deserialize;
use springtime::config::CONFIG_FILE;
use springtime::future::{BoxFuture, FutureExt};
use springtime_di::component_registry::conditional::unregistered_component;
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, injectable, Component};
use std::sync::Arc;

/// A [Deserialize] version of [Target](refinery_core::Target).
#[derive(Clone, Copy, Debug, Deserialize)]
pub enum Target {
    /// Latest version.
    Latest,
    /// User-provided version.
    Version(u32),
    /// Don't run migrations, just update the migration table to latest version.
    Fake,
    /// Don't run migrations, just update the migration table to user-provided version.
    FakeVersion(u32),
}

impl From<Target> for refinery_core::Target {
    fn from(value: Target) -> Self {
        match value {
            Target::Latest => refinery_core::Target::Latest,
            Target::Version(version) => refinery_core::Target::Version(version),
            Target::Fake => refinery_core::Target::Fake,
            Target::FakeVersion(version) => refinery_core::Target::FakeVersion(version),
        }
    }
}

/// Migration configuration.
#[non_exhaustive]
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct MigrationConfig {
    /// Should migrations run on application start.
    pub run_migrations_on_start: bool,
    /// The target version up to which migrate.
    pub target: Target,
    /// Group migrations in a single transaction.
    pub grouped: bool,
    /// Should abort migration process if divergent migrations are found i.e. applied migrations
    /// with the same version but different name or checksum from the ones on the filesystem.
    pub abort_divergent: bool,
    /// Should abort if missing migrations are found i.e. applied migrations that are not found on
    /// the filesystem, or migrations found on filesystem with a version inferior to the last one
    /// applied but not applied
    pub abort_missing: bool,
    /// Table name for migration data.
    pub migration_table_name: String,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            run_migrations_on_start: true,
            target: Target::Latest,
            grouped: false,
            abort_divergent: true,
            abort_missing: true,
            migration_table_name: "refinery_schema_history".to_string(),
        }
    }
}

impl MigrationConfig {
    fn init_from_config() -> Result<Self, ErrorPtr> {
        Config::builder()
            .add_source(File::with_name(CONFIG_FILE).required(false))
            .build()
            .and_then(|config| config.try_deserialize::<MigrationConfigWrapper>())
            .map(|config| config.migration)
            .map_err(|error| Arc::new(error) as ErrorPtr)
    }
}

/// Provider for [MigrationConfig]. The primary instance of the provider will be used to retrieve
/// migration configuration.
#[injectable]
pub trait MigrationConfigProvider {
    /// Provide current config.
    fn config(&self) -> BoxFuture<'_, Result<&MigrationConfig, ErrorPtr>>;
}

#[derive(Component)]
#[component(priority = -128, condition = "unregistered_component::<dyn MigrationConfigProvider + Send + Sync>", constructor = "DefaultMigrationConfigProvider::new")]
struct DefaultMigrationConfigProvider {
    // cached init result
    #[component(ignore)]
    config: Result<MigrationConfig, ErrorPtr>,
}

#[component_alias]
impl MigrationConfigProvider for DefaultMigrationConfigProvider {
    fn config(&self) -> BoxFuture<'_, Result<&MigrationConfig, ErrorPtr>> {
        async {
            match &self.config {
                Ok(config) => Ok(config),
                Err(error) => Err(error.clone()),
            }
        }
        .boxed()
    }
}

impl DefaultMigrationConfigProvider {
    fn new() -> BoxFuture<'static, Result<Self, ErrorPtr>> {
        async {
            Ok(Self {
                config: MigrationConfig::init_from_config(),
            })
        }
        .boxed()
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct MigrationConfigWrapper {
    migration: MigrationConfig,
}
