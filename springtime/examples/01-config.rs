// note: this example assumes you've analyzed the previous one

use springtime::application;
use springtime::config::{ApplicationConfig, ApplicationConfigProvider};
use springtime::runner::ApplicationRunner;
use springtime_di::future::{BoxFuture, FutureExt};
use springtime_di::instance_provider::{ComponentInstancePtr, ErrorPtr};
use springtime_di::{component_alias, Component};

// application config is provided by an ApplicationConfigProvider, which by default, uses
// environment variables and a configuration file (see module documentation)
// to provide your own, register a component implementing this trait, and it should take precedence
// over the default one
#[derive(Component)]
#[component(constructor = "MyApplicationConfigProvider::new")]
struct MyApplicationConfigProvider {
    // this is the cached custom config
    #[component(ignore)]
    config: ApplicationConfig,
}

impl MyApplicationConfigProvider {
    // using a custom constructor allows for caching the config for later use
    fn new() -> BoxFuture<'static, Result<Self, ErrorPtr>> {
        async {
            // start with a default config and override what's needed
            let mut config = ApplicationConfig::default();
            config.install_tracing_logger = false;

            Ok(Self { config })
        }
        .boxed()
    }
}

// register MyApplicationConfigProvider as an ApplicationConfigProvider
#[component_alias]
impl ApplicationConfigProvider for MyApplicationConfigProvider {
    fn config(&self) -> BoxFuture<'_, Result<&ApplicationConfig, ErrorPtr>> {
        async { Ok(&self.config) }.boxed()
    }
}

// this simple component will simply print a part of application config to prove everything works
#[derive(Component)]
struct ConfigPrinterRunner {
    application_config_provider: ComponentInstancePtr<dyn ApplicationConfigProvider + Send + Sync>,
}

//noinspection DuplicatedCode
#[component_alias]
impl ApplicationRunner for ConfigPrinterRunner {
    fn run(&self) -> BoxFuture<'_, Result<(), ErrorPtr>> {
        async {
            let config = self.application_config_provider.config().await?;
            println!("Using built-in logger: {}", config.install_tracing_logger);
            Ok(())
        }
        .boxed()
    }
}

#[tokio::main]
async fn main() {
    let mut application =
        application::create_default().expect("unable to create default application");

    // prints "Using built-in logger: false"
    application.run().await.expect("error running application");
}
