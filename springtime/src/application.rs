//! Core application framework functionality.

use crate::config::ApplicationConfig;
use crate::runner::ApplicationRunnerPtr;
use derive_more::Constructor;
use springtime_di::component_registry::ComponentDefinitionRegistryError;
use springtime_di::factory::{ComponentFactory, ComponentFactoryBuilder};
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
    #[error("Error retrieving runners: {0}")]
    RunnerInjectionError(ComponentInstanceProviderError),
    #[error("Runner error: {0}")]
    RunnerError(ErrorPtr),
    #[error("Cannot retrieve application config: {0}")]
    MissingApplicationConfig(ComponentInstanceProviderError),
    #[error("Error creating default application: {0}")]
    DefaultInitializationError(ComponentDefinitionRegistryError),
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

        for runner in &runners {
            runner.run().await.map_err(|error| {
                error!(%error, "Error running application runner!");
                ApplicationError::RunnerError(error)
            })?;
        }

        Ok(())
    }

    async fn install_logger(
        &mut self,
    ) -> Result<Option<dispatcher::DefaultGuard>, ApplicationError> {
        let config = self
            .instance_provider
            .primary_instance_typed::<ApplicationConfig>()
            .await
            .map_err(ApplicationError::MissingApplicationConfig)?;

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
        let config = self
            .instance_provider
            .primary_instance_typed::<ApplicationConfig>()
            .map_err(ApplicationError::MissingApplicationConfig)?;

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

#[cfg(test)]
mod tests {
    use crate::application::{Application, ApplicationError};
    use crate::config::ApplicationConfig;
    use crate::runner::{ApplicationRunnerPtr, BoxFuture, MockApplicationRunner};
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
            .downcast::<ApplicationConfig>()
            .map(|p| Box::new(p as ComponentInstancePtr<ApplicationConfig>) as Box<dyn Any>)
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

    fn create_instance_provider() -> MockComponentInstanceProvider {
        let mut instance_provider = MockComponentInstanceProvider::new();
        instance_provider
            .expect_primary_instance()
            .with(eq(TypeId::of::<ApplicationConfig>()))
            .returning(|_| {
                async {
                    Ok((
                        ComponentInstancePtr::new(ApplicationConfig {
                            install_tracing_logger: false,
                        }) as ComponentInstanceAnyPtr,
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
