use fts_core::models::Config;
use sqlx::{
    Pool, Sqlite,
    migrate::MigrateDatabase,
    pool::PoolOptions,
    sqlite::{SqliteConnectOptions, SqlitePool},
};
use std::{path::PathBuf, str::FromStr, time::Duration};
use thiserror::Error;

/// Database operations generate errors for multiple reasons, this is a unified
/// error type that our functions can return.
#[derive(Debug, Error)]
pub enum Error {
    /// Error from SQLx operations
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Error in JSON serialization or deserialization
    #[error("deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),

    /// Migration error
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Failure to insert data
    #[error("insertion failed")]
    InsertionFailure,

    /// Conflicting configuration detected
    #[error("inconsistent configuration")]
    InconsistentConfig,

    /// Generic failure with message
    #[error("failure: {0}")]
    Failure(String),
}

/// Storage configuration for the database.
pub enum Storage {
    /// Store data in a file at the specified path
    File(PathBuf),

    /// Store data in memory with the given identifier
    Memory(String),
}

impl Storage {
    fn connection_string(&self) -> String {
        match self {
            Storage::File(path) => format!("sqlite:{}", path.display()),
            Storage::Memory(name) => {
                format!("sqlite::memory:?cache=shared&mode=memory&name={}", name)
            }
        }
    }
}

/// Main database connection manager.
///
/// SQLx provides its own connection pooling with reader/writer capabilities.
#[derive(Clone, Debug)]
pub struct Database {
    pool: Pool<Sqlite>,
    config: Config,
}

impl Database {
    /// Opens a database connection with the specified configuration.
    ///
    /// Creates a new database if one doesn't exist, and applies migrations.
    /// Validates that the provided configuration matches any existing configuration.
    pub async fn open(db: Option<&PathBuf>, config: Option<Config>) -> Result<Self, Error> {
        let storage = db
            .map(|path| Storage::File(path.clone()))
            .unwrap_or(Storage::Memory("orderbook".to_owned()));

        let database = open_db(storage, config).await?;

        Ok(database)
    }

    /// Get a connection from the pool.
    pub async fn acquire(&self) -> Result<sqlx::pool::PoolConnection<Sqlite>, Error> {
        Ok(self.pool.acquire().await?)
    }

    /// Get a transaction from the pool.
    pub async fn begin(&self) -> Result<sqlx::Transaction<'_, Sqlite>, Error> {
        Ok(self.pool.begin().await?)
    }

    /// Commit a transaction.
    pub async fn commit(&self, tx: sqlx::Transaction<'_, Sqlite>) -> Result<(), Error> {
        tx.commit().await?;
        Ok(())
    }

    /// Rollback a transaction.
    pub async fn rollback(&self, tx: sqlx::Transaction<'_, Sqlite>) -> Result<(), Error> {
        tx.rollback().await?;
        Ok(())
    }

    /// Get a reference to the flow trading configuration
    pub fn config(&self) -> &Config {
        &self.config
    }
}

/// Creates an instance of Database with an SQLx connection pool.
async fn open_db(storage: Storage, config: Option<Config>) -> Result<Database, Error> {
    let connection_str = storage.connection_string();

    // Create database if it doesn't exist (for file-based storage)
    if let Storage::File(_) = &storage {
        if !Sqlite::database_exists(&connection_str)
            .await
            .unwrap_or(false)
        {
            Sqlite::create_database(&connection_str).await?;
        }
    }

    // Configure SQLite connection options
    let connect_options = SqliteConnectOptions::from_str(&connection_str)?
        // TODO: validate these settings and possibly add to them. Some helpful resources:
        // * https://lobste.rs/s/fxkk7v/why_does_sqlite_production_have_such_bad
        // * https://kerkour.com/sqlite-for-servers
        // * https://gcollazo.com/optimal-sqlite-settings-for-django/
        // * https://lobste.rs/s/rvsgqy/gotchas_with_sqlite_production
        // * https://blog.pecar.me/sqlite-prod
        .pragma("journal_mode", "WAL")
        .pragma("busy_timeout", "5000")
        .pragma("synchronous", "NORMAL")
        .pragma("foreign_keys", "true")
        .pragma("mmap_size", "134217728")
        .pragma("journal_size_limit", "27103364")
        .pragma("cache_size", "2000")
        .create_if_missing(true);

    // Create the connection pool
    let pool_options = PoolOptions::<Sqlite>::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3));

    let pool = pool_options.connect_with(connect_options).await?;

    // Run migrations
    run_migrations(&pool).await?;

    // Handle configuration
    let stored_config: Option<Config> =
        sqlx::query_scalar!("SELECT data FROM config WHERE id = 0 LIMIT 1")
            .fetch_optional(&pool)
            .await?
            .map(|json_value: String| serde_json::from_str(&json_value))
            .transpose()?;

    let actual_config = if let Some(stored_config) = stored_config {
        if config.is_some_and(|c| c != stored_config) {
            return Err(Error::InconsistentConfig);
        }
        stored_config
    } else if let Some(config) = config {
        let json = serde_json::to_string(&config)?;

        sqlx::query!(
            "INSERT INTO config (id, data) VALUES (0, ?1) ON CONFLICT (id) DO UPDATE SET data = excluded.data",
            json
        )
        .execute(&pool)
        .await?;

        config
    } else {
        return Err(Error::Failure("no configuration specified".to_string()));
    };

    Ok(Database {
        pool,
        config: actual_config,
    })
}

/// Run database migrations using SQLx's migration system
async fn run_migrations(pool: &SqlitePool) -> Result<(), Error> {
    // Using SQLx's migration system
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| Error::Migration(e))
}
