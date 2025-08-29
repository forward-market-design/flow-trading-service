#![warn(missing_docs)]
// Note: this overwrites the link in the README to point to the rust docs of the fts-sqlite crate.
//! [fts_core]: https://docs.rs/fts_core/latest/fts_core/index.html
//! [fts_axum]: https://docs.rs/fts_axum/latest/fts_axum/index.html
//! [fts_solver]: https://docs.rs/fts_solver/latest/fts_solver/index.html
//! [fts_sqlite]: https://docs.rs/fts_sqlite/latest/fts_sqlite/index.html
#![doc = include_str!("../README.md")]

use sqlx::sqlite;
use std::{str::FromStr, time::Duration};
use tokio::try_join;

pub mod config;
mod r#impl;
pub mod types;

use config::SqliteConfig;

/// SQLite database implementation for flow trading repositories.
///
/// This struct provides separate reader and writer connection pools to a SQLite database,
/// implementing all the repository traits defined in `fts-core`. The separation of read
/// and write connections allows for better concurrency control and follows SQLite best
/// practices for Write-Ahead Logging (WAL) mode.
///
/// # Connection Management
///
/// - `reader`: A connection pool for read operations, allowing concurrent reads
/// - `writer`: A single-connection pool for write operations, ensuring serialized writes
///
/// # Example
///
/// ```no_run
/// # use fts_sqlite::{Db, config::SqliteConfig, types::DateTime};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = SqliteConfig::default();
/// let db = Db::open(&config).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Db {
    /// Connection pool for read operations
    pub reader: sqlx::Pool<sqlx::Sqlite>,
    /// Connection pool for write operations (limited to 1 connection)
    pub writer: sqlx::Pool<sqlx::Sqlite>,
}

impl Db {
    /// Open a connection to the specified SQLite database.
    ///
    /// Creates a new database if one doesn't exist (when `create_if_missing` is true),
    /// applies all pending migrations, and ensures the batch table is initialized.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration specifying database path and creation options
    ///
    /// # Database Configuration
    ///
    /// The database is configured with the following settings for optimal performance:
    /// - WAL mode for better concurrency
    /// - Foreign keys enabled for referential integrity
    /// - Optimized cache and memory settings for flow trading workloads
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` if:
    /// - Database connection fails
    /// - Migrations fail to apply
    pub async fn open(config: &SqliteConfig) -> Result<Self, sqlx::Error> {
        let db_path = config
            .database_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned());

        // Use the same hardcoded pragmas as the original open() method
        let options =
            sqlite::SqliteConnectOptions::from_str(db_path.as_deref().unwrap_or(":memory:"))?
                .busy_timeout(Duration::from_secs(5))
                .foreign_keys(true)
                .journal_mode(sqlite::SqliteJournalMode::Wal)
                .synchronous(sqlite::SqliteSynchronous::Normal)
                .pragma("cache_size", "1000000000")
                .pragma("journal_size_limit", "27103364")
                .pragma("mmap_size", "134217728")
                .pragma("temp_store", "memory")
                .create_if_missing(config.create_if_missing);

        // TODO: setting read_only(true) on the reader seems to also lock the writer, at least when using :memory:. Need to investigate.
        let reader = sqlite::SqlitePoolOptions::new().connect_with(options.clone());
        let writer = sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options);

        let (reader, writer) = try_join!(reader, writer)?;

        // Run any pending migrations before returning
        sqlx::migrate!("./schema").run(&writer).await?;

        Ok(Self { reader, writer })
    }
}
