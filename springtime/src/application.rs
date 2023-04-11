//! Core application framework functionality.

use crate::runner::ApplicationRunnerPtr;
use derive_more::Constructor;
use springtime_di::instance_provider::{
    ComponentInstanceProvider, ComponentInstanceProviderError, ErrorPtr,
    TypedComponentInstanceProvider,
};
use thiserror::Error;
use tracing::info;

#[derive(Clone, Error, Debug)]
pub enum ApplicationError {
    #[error("Error retrieving runners: {0}")]
    RunnerInjectionError(ComponentInstanceProviderError),
    #[error("Runner error: {0}")]
    RunnerError(ErrorPtr),
}

/// Helper trait for component instance provider which can either be sync or async.
#[cfg(feature = "async")]
pub trait ApplicationComponentInstanceProvider: ComponentInstanceProvider + Send + Sync {}

/// Helper trait for component instance provider which can either be sync or async.
#[cfg(not(feature = "async"))]
pub trait ApplicationComponentInstanceProvider: ComponentInstanceProvider {}

#[cfg(feature = "async")]
impl<T: ComponentInstanceProvider + Send + Sync + ?Sized> ApplicationComponentInstanceProvider
    for T
{
}

#[cfg(not(feature = "async"))]
impl<T: ComponentInstanceProvider + ?Sized> ApplicationComponentInstanceProvider for T {}

/// Main entrypoint for the application. Bootstraps the application and runs
/// [ApplicationRunners](crate::runner::ApplicationRunner).
#[derive(Constructor)]
pub struct Application<CIP: ApplicationComponentInstanceProvider> {
    instance_provider: CIP,
}

impl<CIP: ApplicationComponentInstanceProvider> Application<CIP> {
    #[cfg(feature = "async")]
    pub async fn run(&mut self) -> Result<(), ApplicationError> {
        info!("Searching for application runners...");

        let mut runners = self
            .instance_provider
            .instances_typed::<ApplicationRunnerPtr>()
            .await
            .map_err(ApplicationError::RunnerInjectionError)?;

        runners.sort_unstable_by_key(|runner| -runner.priority());

        info!("Running application runners...");

        for runner in &runners {
            runner.run().await.map_err(ApplicationError::RunnerError)?;
        }

        Ok(())
    }

    #[cfg(not(feature = "async"))]
    pub fn run(&mut self) -> Result<(), ApplicationError> {
        info!("Searching for application runners...");

        let mut runners = self
            .instance_provider
            .instances_typed::<ApplicationRunnerPtr>()
            .map_err(ApplicationError::RunnerInjectionError)?;

        runners.sort_unstable_by_key(|runner| -runner.priority());

        info!("Running application runners...");

        for runner in &runners {
            runner.run().map_err(ApplicationError::RunnerError)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::application::{Application, ApplicationError};
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

    #[tokio::test]
    async fn should_return_injector_error() {
        let type_id = TypeId::of::<ApplicationRunnerPtr>();

        let mut instance_provider = MockComponentInstanceProvider::new();
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

        let mut instance_provider = MockComponentInstanceProvider::new();
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
