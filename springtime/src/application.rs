//! Core application framework functionality.

use crate::config::ApplicationConfigProvider;
use crate::runner::ApplicationRunnerPtr;
use derive_more::Constructor;
#[cfg(feature = "async")]
use futures::future::try_join_all;
use springtime_di::component_registry::ComponentDefinitionRegistryError;
use springtime_di::factory::{ComponentFactory, ComponentFactoryBuilder};
#[cfg(feature = "async")]
use springtime_di::instance_provider::ComponentInstancePtr;
use springtime_di::instance_provider::{
    ComponentInstanceProvider, ComponentInstanceProviderError, ErrorPtr,
    TypedComponentInstanceProvider,
};
use thiserror::Error;
use tracing::{dispatcher, error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone, Error, Debug)]
pub enum ApplicationError {
    /// There was an error retrieving application runners from the component instance factory.
    #[error("Error retrieving runners: {0}")]
    RunnerInjectionError(ComponentInstanceProviderError),
    /// A runner returned an error.
    #[error("Runner error: {0}")]
    RunnerError(ErrorPtr),
    /// Cannot find any [ApplicationConfigProvider].
    #[error("Cannot retrieve application config provider: {0}")]
    MissingApplicationConfigProvider(ComponentInstanceProviderError),
    /// An error occurred while creating the default [Application].
    #[error("Error creating default application: {0}")]
    DefaultInitializationError(ComponentDefinitionRegistryError),
    /// [ApplicationConfigProvider] returned an error.
    #[error("Cannot retrieve application config: {0}")]
    CannotRetrieveApplicationConfig(ErrorPtr),
}

/// Main entrypoint for the application. Bootstraps the application and runs
/// [ApplicationRunners](crate::runner::ApplicationRunner).
#[derive(Constructor)]
#[cfg(feature = "async")]
pub struct Application<CIP: ComponentInstanceProvider + Send + Sync> {
    instance_provider: CIP,
}

/// Main entrypoint for the application. Bootstraps the application and runs
/// [ApplicationRunners](crate::runner::ApplicationRunner).
#[derive(Constructor)]
#[cfg(not(feature = "async"))]
pub struct Application<CIP: ComponentInstanceProvider> {
    instance_provider: CIP,
}

#[cfg(feature = "async")]
impl<CIP: ComponentInstanceProvider + Send + Sync> Application<CIP> {
    pub async fn run(&mut self) -> Result<(), ApplicationError> {
        let _logger = self.install_logger().await?;

        info!("Searching for application runners...");

        let mut runners = self
            .instance_provider
            .instances_typed::<ApplicationRunnerPtr>()
            .await
            .map_err(|error| {
                error!(%error, "Error retrieving application runners!");
                ApplicationError::RunnerInjectionError(error)
            })?;

        runners.sort_unstable_by_key(|runner| -runner.priority());

        info!("Running application runners...");

        let mut current_runner_index = 0;
        while current_runner_index < runners.len() {
            current_runner_index += run_grouped_by_priority(&runners[current_runner_index..])
                .await
                .map_err(|error| {
                    error!(%error, "Error running application runner!");
                    ApplicationError::RunnerError(error)
                })?;
        }

        Ok(())
    }

    async fn install_logger(
        &mut self,
    ) -> Result<Option<dispatcher::DefaultGuard>, ApplicationError> {
        let config_provider = self
            .instance_provider
            .primary_instance_typed::<dyn ApplicationConfigProvider + Send + Sync>()
            .await
            .map_err(ApplicationError::MissingApplicationConfigProvider)?;

        let config = config_provider
            .config()
            .await
            .map_err(ApplicationError::CannotRetrieveApplicationConfig)?;

        if !config.install_tracing_logger {
            return Ok(None);
        }

        Ok(Some(
            tracing_subscriber::registry()
                .with(EnvFilter::from_default_env())
                .with(fmt::layer())
                .set_default(),
        ))
    }
}

#[cfg(not(feature = "async"))]
impl<CIP: ComponentInstanceProvider> Application<CIP> {
    pub fn run(&mut self) -> Result<(), ApplicationError> {
        let _logger = self.install_logger()?;

        info!("Searching for application runners...");

        let mut runners = self
            .instance_provider
            .instances_typed::<ApplicationRunnerPtr>()
            .map_err(|error| {
                error!(%error, "Error retrieving application runners!");
                ApplicationError::RunnerInjectionError(error)
            })?;

        runners.sort_unstable_by_key(|runner| -runner.priority());

        info!("Running application runners...");

        for runner in &runners {
            runner.run().map_err(|error| {
                error!(%error, "Error running application runner!");
                ApplicationError::RunnerError(error)
            })?;
        }

        Ok(())
    }

    fn install_logger(&mut self) -> Result<Option<dispatcher::DefaultGuard>, ApplicationError> {
        #[cfg(feature = "threadsafe")]
        type ProviderType = dyn ApplicationConfigProvider + Send + Sync;

        #[cfg(not(feature = "threadsafe"))]
        type ProviderType = dyn ApplicationConfigProvider;

        let config_provider = self
            .instance_provider
            .primary_instance_typed::<ProviderType>()
            .map_err(ApplicationError::MissingApplicationConfigProvider)?;

        let config = config_provider
            .config()
            .map_err(ApplicationError::CannotRetrieveApplicationConfig)?;

        if !config.install_tracing_logger {
            return Ok(None);
        }

        Ok(Some(
            tracing_subscriber::registry()
                .with(EnvFilter::from_default_env())
                .with(fmt::layer())
                .set_default(),
        ))
    }
}

/// Creates an [Application] with a sensible default configuration.
pub fn create_default() -> Result<Application<ComponentFactory>, ApplicationError> {
    let component_factory = ComponentFactoryBuilder::new()
        .map_err(ApplicationError::DefaultInitializationError)?
        .build();

    Ok(Application::new(component_factory))
}

// this could be replaced by group_by() from itertools, but it doesn't impl Send
#[cfg(feature = "async")]
async fn run_grouped_by_priority(
    runners: &[ComponentInstancePtr<ApplicationRunnerPtr>],
) -> Result<usize, ErrorPtr> {
    // note: assuming runners are sorted by priority
    let current_priority = runners[0].priority();
    let first_new_priority_index = runners
        .iter()
        .enumerate()
        .find(|(_, entry)| entry.priority() != current_priority)
        .map(|(index, _)| index)
        .unwrap_or(runners.len());

    try_join_all(
        runners[..first_new_priority_index]
            .iter()
            .map(|runner| runner.run()),
    )
    .await
    .map(move |_| first_new_priority_index)
}

#[cfg(test)]
mod tests {
    use crate::application::{Application, ApplicationError};
    use crate::config::{ApplicationConfig, ApplicationConfigProvider};
    use crate::future::{BoxFuture, MockApplicationRunner};
    use crate::runner::{ApplicationRunnerPtr, MockApplicationRunner};
    use mockall::mock;
    use mockall::predicate::*;
    use springtime_di::future::FutureExt;
    use springtime_di::instance_provider::{
        CastFunction, ComponentInstanceAnyPtr, ComponentInstanceProvider,
        ComponentInstanceProviderError, ComponentInstancePtr, ErrorPtr,
    };
    use std::any::{Any, TypeId};
    use std::sync::Arc;

    fn mock_cast(
        instance: ComponentInstanceAnyPtr,
    ) -> Result<Box<dyn Any>, ComponentInstanceAnyPtr> {
        instance
            .downcast::<MockApplicationRunner>()
            .map(|p| Box::new(p as ComponentInstancePtr<ApplicationRunnerPtr>) as Box<dyn Any>)
    }

    fn config_cast(
        instance: ComponentInstanceAnyPtr,
    ) -> Result<Box<dyn Any>, ComponentInstanceAnyPtr> {
        instance
            .downcast::<MockApplicationConfigProvider>()
            .map(|p| {
                Box::new(p as ComponentInstancePtr<dyn ApplicationConfigProvider + Send + Sync>)
                    as Box<dyn Any>
            })
    }

    mock! {
        ComponentInstanceProvider {}

        impl ComponentInstanceProvider for ComponentInstanceProvider {
            fn primary_instance(
                &mut self,
                type_id: TypeId,
            ) -> BoxFuture<
                '_,
                Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError>,
            >;

            fn instances(
                &mut self,
                type_id: TypeId,
            ) -> BoxFuture<
                '_,
                Result<Vec<(ComponentInstanceAnyPtr, CastFunction)>, ComponentInstanceProviderError>,
            >;

            fn instance_by_name(
                &mut self,
                name: &str,
                type_id: TypeId,
            ) -> BoxFuture<
                '_,
                Result<(ComponentInstanceAnyPtr, CastFunction), ComponentInstanceProviderError>,
            >;
        }
    }

    const CONFIG: ApplicationConfig = ApplicationConfig {
        install_tracing_logger: false,
    };

    #[derive(Default)]
    struct MockApplicationConfigProvider;

    impl ApplicationConfigProvider for MockApplicationConfigProvider {
        fn config(&self) -> BoxFuture<'_, Result<&ApplicationConfig, ErrorPtr>> {
            async { Ok(&CONFIG) }.boxed()
        }
    }

    fn create_instance_provider() -> MockComponentInstanceProvider {
        let application_config_provider =
            ComponentInstancePtr::new(MockApplicationConfigProvider::default());

        let mut instance_provider = MockComponentInstanceProvider::new();
        instance_provider
            .expect_primary_instance()
            .with(eq(
                TypeId::of::<dyn ApplicationConfigProvider + Send + Sync>(),
            ))
            .returning(move |_| {
                let application_config_provider = application_config_provider.clone();
                async move {
                    Ok((
                        application_config_provider.clone() as ComponentInstanceAnyPtr,
                        config_cast as CastFunction,
                    ))
                }
                .boxed()
            });

        instance_provider
    }

    #[tokio::test]
    async fn should_return_injector_error() {
        let type_id = TypeId::of::<ApplicationRunnerPtr>();

        let mut instance_provider = create_instance_provider();
        instance_provider
            .expect_instances()
            .with(eq(type_id))
            .times(1)
            .returning(move |_| {
                async move { Err(ComponentInstanceProviderError::NoPrimaryInstance(type_id)) }
                    .boxed()
            });

        let mut application = Application::new(instance_provider);
        assert!(matches!(
            application.run().await.unwrap_err(),
            ApplicationError::RunnerInjectionError(_)
        ));
    }

    #[tokio::test]
    async fn should_return_runner_error() {
        let type_id = TypeId::of::<ApplicationRunnerPtr>();

        let mut instance_provider = create_instance_provider();
        instance_provider
            .expect_instances()
            .with(eq(type_id))
            .times(1)
            .returning(|_| {
                let mut runner = MockApplicationRunner::new();
                runner.expect_run().returning(|| {
                    async {
                        Err(Arc::new(ComponentInstanceProviderError::NoPrimaryInstance(
                            TypeId::of::<i8>(),
                        )) as ErrorPtr)
                    }
                    .boxed()
                });
                runner.expect_priority().return_const(0);

                async {
                    Ok(vec![(
                        ComponentInstancePtr::new(runner) as ComponentInstanceAnyPtr,
                        mock_cast as CastFunction,
                    )])
                }
                .boxed()
            });

        let mut application = Application::new(instance_provider);
        assert!(matches!(
            application.run().await.unwrap_err(),
            ApplicationError::RunnerError(_)
        ));
    }
}
