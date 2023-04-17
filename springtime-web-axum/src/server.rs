//! Core server-related functionality.

use crate::config::ServerConfig;
use hyper::server::conn::AddrIncoming;
use hyper::server::Builder;
use hyper::Error as HyperError;
use springtime_di::component_registry::conditional::unregistered_component;
use springtime_di::future::{BoxFuture, FutureExt};
use springtime_di::{component_alias, injectable, Component};
use std::net::AddrParseError;
use thiserror::Error;

/// Errors related to bootstrapping servers.
#[derive(Error, Debug)]
pub enum ServerBootstrapError {
    #[error("Error parsing listen address: {0}")]
    ListenAddressParseError(AddrParseError),
    #[error("Error binding server: {0}")]
    BindError(#[source] HyperError),
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
