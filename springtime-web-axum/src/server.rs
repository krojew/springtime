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
}

#[component_alias]
impl ApplicationRunner for ServerRunner {
    fn run(&self) -> BoxFuture<'_, Result<(), ErrorPtr>> {
        async {
            let config = self.config_provider.config().await?;
            let servers = self
                .create_servers(config)
                .await
                .map_err(|error| Arc::new(error) as ErrorPtr)?;

            try_join_all(servers.into_iter()).await.map(|_| ())
        }
        .boxed()
    }
}

impl ServerRunner {
    async fn create_server(
        &self,
        config: &ServerConfig,
        server_name: &str,
    ) -> Result<impl Future<Output = Result<(), ErrorPtr>>, ServerBootstrapError> {
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
                    .await
                    .map_err(|error| Arc::new(error) as ErrorPtr)
            })
    }

    async fn create_servers(
        &self,
        config: &WebConfig,
    ) -> Result<Vec<impl Future<Output = Result<(), ErrorPtr>>>, ServerBootstrapError> {
        let mut result = Vec::with_capacity(config.servers.len());
        for (server_name, config) in config.servers.iter() {
            result.push(self.create_server(config, server_name).await?);
        }

        Ok(result)
    }
}
