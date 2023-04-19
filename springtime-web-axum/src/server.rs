//! Core server-related functionality.

use crate::config::{ServerConfig, WebConfig, WebConfigProvider};
use crate::router::RouterBootstrap;
use futures::future::try_join_all;
use hyper::server::conn::AddrIncoming;
use hyper::server::Builder;
use hyper::Error as HyperError;
use springtime::runner::ApplicationRunner;
use springtime::runner::{BoxFuture, FutureExt};
use springtime_di::component_registry::conditional::unregistered_component;
use springtime_di::instance_provider::{ComponentInstancePtr, ErrorPtr};
use springtime_di::{component_alias, injectable, Component};
use std::future::Future;
use std::net::AddrParseError;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::watch::{channel, Receiver, Sender};
use tracing::{debug, info};

pub type ShutdownSignalSender = Sender<()>;

/// Errors related to bootstrapping servers.
#[derive(Error, Debug)]
pub enum ServerBootstrapError {
    #[error("Error parsing listen address: {0}")]
    ListenAddressParseError(AddrParseError),
    #[error("Error binding server: {0}")]
    BindError(#[source] HyperError),
    #[error("Error configuring router: {0}")]
    RouterError(#[source] ErrorPtr),
}

/// Trait for components responsible for creating web servers from
/// [ServerConfig](crate::config::ServerConfig). Create a component implementing this trait to
/// override the default bootstrap.
#[injectable]
pub trait ServerBootstrap {
    /// Create a [Builder] which will them be used to create a web server.
    fn bootstrap_server(
        &self,
        config: &ServerConfig,
    ) -> BoxFuture<'_, Result<Builder<AddrIncoming>, ServerBootstrapError>>;
}

#[derive(Component)]
#[component(priority = -128, condition = "unregistered_component::<dyn ServerBootstrap + Send + Sync>")]
struct DefaultServerBootstrap;

#[component_alias]
impl ServerBootstrap for DefaultServerBootstrap {
    fn bootstrap_server(
        &self,
        config: &ServerConfig,
    ) -> BoxFuture<'_, Result<Builder<AddrIncoming>, ServerBootstrapError>> {
        let listen_address = config.listen_address.clone();

        async move {
            axum::Server::try_bind(
                &listen_address
                    .parse()
                    .map_err(ServerBootstrapError::ListenAddressParseError)?,
            )
            .map_err(ServerBootstrapError::BindError)
        }
        .boxed()
    }
}

#[derive(Component)]
struct ServerRunner {
    server_bootstrap: ComponentInstancePtr<dyn ServerBootstrap + Send + Sync>,
    router_bootstrap: ComponentInstancePtr<dyn RouterBootstrap + Send + Sync>,
    config_provider: ComponentInstancePtr<dyn WebConfigProvider + Send + Sync>,
    shutdown_signal_source: Option<ComponentInstancePtr<dyn ShutdownSignalSource + Send + Sync>>,
}

#[component_alias]
impl ApplicationRunner for ServerRunner {
    fn run(&self) -> BoxFuture<'_, Result<(), ErrorPtr>> {
        async {
            info!("Starting servers...");

            let (tx, rx) = channel(());
            if let Some(shutdown_signal_source) = &self.shutdown_signal_source {
                shutdown_signal_source.register_shutdown(tx)?;
            }

            let config = self.config_provider.config().await?;
            let servers = self
                .create_servers(config, rx)
                .await
                .map_err(|error| Arc::new(error) as ErrorPtr)?;

            info!("Running {} servers...", servers.len());

            try_join_all(servers.into_iter()).await?;

            info!("All servers stopped.");

            Ok(())
        }
        .boxed()
    }
}

impl ServerRunner {
    async fn create_server(
        &self,
        config: &ServerConfig,
        server_name: &str,
        mut shutdown_receiver: Receiver<()>,
    ) -> Result<impl Future<Output = Result<(), ErrorPtr>>, ServerBootstrapError> {
        debug!(server_name, "Creating new server.");

        let router = self
            .router_bootstrap
            .bootstrap_router(server_name)
            .map_err(ServerBootstrapError::RouterError)?;

        self.server_bootstrap
            .bootstrap_server(config)
            .await
            .map(move |builder| async move {
                builder
                    .serve(router.into_make_service())
                    .with_graceful_shutdown(async move {
                        let _ = shutdown_receiver.changed().await;
                    })
                    .await
                    .map_err(|error| Arc::new(error) as ErrorPtr)
            })
    }

    async fn create_servers(
        &self,
        config: &WebConfig,
        shutdown_receiver: Receiver<()>,
    ) -> Result<Vec<impl Future<Output = Result<(), ErrorPtr>>>, ServerBootstrapError> {
        let mut result = Vec::with_capacity(config.servers.len());
        for (server_name, config) in config.servers.iter() {
            result.push(
                self.create_server(config, server_name, shutdown_receiver.clone())
                    .await?,
            );
        }

        Ok(result)
    }
}

/// Source for gracefully shutting down the server runner with all running servers. Only the primary
/// instance is taken into account.
#[injectable]
pub trait ShutdownSignalSource {
    /// Takes given signal sender to add custom shutdown signaling logic.
    fn register_shutdown(&self, shutdown_sender: ShutdownSignalSender) -> Result<(), ErrorPtr>;
}
