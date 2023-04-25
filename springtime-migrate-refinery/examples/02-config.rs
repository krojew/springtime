// note: this example assumes you've analyzed the previous one

use springtime::application;
use springtime::future::{BoxFuture, FutureExt};
use springtime_di::instance_provider::ErrorPtr;
use springtime_di::{component_alias, Component};
use springtime_migrate_refinery::config::{MigrationConfig, MigrationConfigProvider};

// config is provided by a MigrationConfigProvider, which by default, uses a configuration file (see
// module documentation)
// to provide your own, register a component implementing this trait, and it should take precedence
// over the default one
#[derive(Component)]
#[component(constructor = "MyMigrationConfigProvider::new")]
struct MyMigrationConfigProvider {
    // this is the cached custom config
    #[component(ignore)]
    config: MigrationConfig,
}

impl MyMigrationConfigProvider {
    fn new() -> BoxFuture<'static, Result<Self, ErrorPtr>> {
        async {
            // start with a default config and override what's needed
            let mut config = MigrationConfig::default();
            config.abort_missing = false;

            Ok(Self { config })
        }
        .boxed()
    }
}

// register MyMigrationConfigProvider as a MigrationConfigProvider
#[component_alias]
impl MigrationConfigProvider for MyMigrationConfigProvider {
    fn config(&self) -> BoxFuture<'_, Result<&MigrationConfig, ErrorPtr>> {
        async { Ok(&self.config) }.boxed()
    }
}

#[tokio::main]
async fn main() {
    let mut application = application::create_default().expect("unable to create application");
    application.run().await.expect("error running application");
}
