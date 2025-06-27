//! Application configuration management.
//!
//! This module handles loading and merging configuration from multiple sources
//! with a clear precedence order. Configuration can come from default values,
//! configuration files, and environment variables.

use crate::{Cli, schedule::Scheduler};
use serde::{Deserialize, Serialize};

/// The main application configuration that composes all component configs
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AppConfig {
    /// Web server configuration (bind address, ports, pagination limits, etc.)
    #[serde(default)]
    pub server: fts_axum::config::AxumConfig,

    /// Database configuration (connection strings, pool settings, etc.)
    #[serde(default)]
    pub database: fts_sqlite::config::SqliteConfig,

    /// Batch auction scheduling configuration
    #[serde(default)]
    pub schedule: Scheduler,
}

impl AppConfig {
    /// Load configuration from multiple sources with precedence:
    /// 1. Environment variables (highest priority)
    /// 2. Config file given by the CLI
    /// 3. Default values (lowest priority)
    ///
    /// Environment variables are mapped using the pattern:
    /// `APP_<SECTION>__<KEY>` maps to `<section>.<key>`
    ///
    /// # Examples
    ///
    /// ```bash
    /// # Set database URL via environment variable
    /// export APP_DATABASE__URL="sqlite:///data/app.db"
    ///
    /// # Set server bind address
    /// export APP_SERVER__BIND_ADDRESS="0.0.0.0:3000"
    ///
    /// # Set scheduling interval
    /// export APP_SCHEDULE__EVERY="1h"
    /// ```
    pub fn load(cli: &Cli) -> anyhow::Result<Self> {
        let mut config = config::Config::builder();

        // Start with default values
        config = config.add_source(config::Config::try_from(&Self::default())?);

        // Layer on config file if it is specified and exists
        if let Some(path) = &cli.config {
            if path.exists() {
                config = config.add_source(config::File::from(path.as_path()))
            } else {
                return Err(anyhow::anyhow!(
                    "Config file {} does not exist",
                    path.display()
                ));
            }
        }

        // Override with environment variables
        // This maps APP_SERVER__BIND_ADDRESS to server.bind_address
        config = config.add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__")
                .try_parsing(true),
        );

        let built_config = config.build()?;
        built_config.try_deserialize().map_err(Into::into)
    }
}
