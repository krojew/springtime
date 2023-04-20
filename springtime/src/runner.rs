//! Runners executing actual application logic.

#[cfg(feature = "async")]
use crate::future::BoxFuture;
#[cfg(test)]
use mockall::automock;
use springtime_di::injectable;
pub use springtime_di::instance_provider::ErrorPtr;

#[cfg(feature = "threadsafe")]
pub type ApplicationRunnerPtr = dyn ApplicationRunner + Send + Sync;

#[cfg(not(feature = "threadsafe"))]
pub type ApplicationRunnerPtr = dyn ApplicationRunner;

/// Runs application logic. Runners are run by the [Application](crate::application::Application)
/// and are discovered by the dependency injection framework. If the `async` feature is enabled,
/// runners with the same priority are ran concurrently.
#[injectable]
#[cfg_attr(test, automock)]
pub trait ApplicationRunner {
    #[cfg(feature = "async")]
    /// Runs any application code.
    fn run(&self) -> BoxFuture<'_, Result<(), ErrorPtr>>;

    #[cfg(not(feature = "async"))]
    /// Runs any application code.
    fn run(&self) -> Result<(), ErrorPtr>;

    /// Returns the priority for this runner. Higher priorities get run first. Default 0.
    fn priority(&self) -> i8 {
        0
    }
}
