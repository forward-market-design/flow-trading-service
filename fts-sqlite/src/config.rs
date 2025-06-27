//! Configuration types for the SQLite database connection.
//!
//! This module provides configuration options for establishing and managing
//! SQLite database connections.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for SQLite database connections.
///
/// This struct controls how the database connection is established.
///
/// # Examples
///
/// ```
/// use fts_sqlite::config::SqliteConfig;
/// use std::path::PathBuf;
///
/// // In-memory database (default)
/// let config = SqliteConfig::default();
///
/// // File-based database
/// let config = SqliteConfig {
///     database_path: Some(PathBuf::from("flow_trading.db")),
///     create_if_missing: true,
/// };
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SqliteConfig {
    /// Database file path. If None, uses in-memory database
    pub database_path: Option<PathBuf>,

    /// Whether to create the database if it doesn't exist
    #[serde(default = "default_true")]
    pub create_if_missing: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            database_path: None,
            create_if_missing: true,
        }
    }
}
