//! Runners executing actual application logic.

#[cfg(test)]
use mockall::automock;
#[cfg(feature = "async")]
pub use springtime_di::future::BoxFuture;
use springtime_di::injectable;
pub use springtime_di::instance_provider::ErrorPtr;

#[cfg(feature = "threadsafe")]
pub type ApplicationRunnerPtr = dyn ApplicationRunner + Send + Sync;

#[cfg(not(feature = "threadsafe"))]
pub type ApplicationRunnerPtr = dyn ApplicationRunner;

/// Runs application logic. Runners are ran by the [Application](crate::application::Application)
/// and are discovered by the dependency injection framework.
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
