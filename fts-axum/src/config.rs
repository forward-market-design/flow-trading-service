//! Configuration types for the Axum HTTP server.
//!
//! This module provides configuration options for the REST API server,
//! including network binding and pagination settings.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Configuration for the Axum HTTP server.
///
/// # Examples
///
/// ```
/// use fts_axum::config::AxumConfig;
/// use std::net::SocketAddr;
///
/// // Use default configuration
/// let config = AxumConfig::default();
///
/// // Custom configuration
/// let config = AxumConfig {
///     bind_address: "127.0.0.1:3000".parse().unwrap(),
///     page_limit: 50,
///     auto_solve: false,
/// };
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AxumConfig {
    /// The address to bind the server to
    #[serde(default = "default_bind_address")]
    pub bind_address: SocketAddr,

    /// The page limit for paginated responses
    #[serde(default = "default_page_limit")]
    pub page_limit: usize,

    /// A flag that, if true, will execute a full auction solve on every bid update
    #[serde(default)]
    pub auto_solve: bool,
}

fn default_bind_address() -> SocketAddr {
    "0.0.0.0:8080".parse().unwrap()
}

fn default_page_limit() -> usize {
    100
}

impl Default for AxumConfig {
    fn default() -> Self {
        Self {
            bind_address: default_bind_address(),
            page_limit: default_page_limit(),
            auto_solve: Default::default(),
        }
    }
}
