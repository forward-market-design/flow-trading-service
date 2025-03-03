use crate::config::Config;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use refinery::Runner;
use rusqlite::OpenFlags;
use std::{ops::DerefMut, path::PathBuf};
use thiserror::Error;

// Database operations generate errors for multiple reasons, this is a unified
// error type that our functions can return.
#[derive(Debug, Error)]
pub enum Error {
    #[error("pool error: {0}")]
    ConnectionPool(#[from] r2d2::Error),
    #[error("deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),
    #[error("migration error: {0}")]
    Migration(#[from] refinery::Error),
    #[error("sql error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("insertion failed")]
    InsertionFailure,
    #[error("inconsistent configuration")]
    InconsistentConfig,
    #[error("failure: {0}")]
    Failure(String),
}

// This is for the "driver" to specify where to load the data from
pub enum Storage {
    File(PathBuf),
    Memory(String),
}

// Sqlite does not have parallel writes, so we create two separate connection
// pools. The reader has unlimited connections, while the writer is capped to
// one. Sqlite has its own mutex shenanigans to make that work out. Modules
// that interact with the database will clone a "master" ReaderWriter.
#[derive(Clone, Debug)]
pub struct Database {
    reader: Pool<SqliteConnectionManager>,
    writer: Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn open(db: Option<&PathBuf>, config: Option<&Config>) -> Result<Self, Error> {
        let storage = db
            .map(|path| Storage::File(path.clone()))
            .unwrap_or(Storage::Memory("orderbook".to_owned()));

        let database = open_rw(storage, Some(crate::embedded::migrations::runner()))?;

        let conn = database.connect(true)?;
        let stored_config = Config::get(&conn)?;

        if let Some(stored_config) = stored_config {
            if let Some(config) = config {
                if stored_config != *config {
                    return Err(Error::InconsistentConfig);
                }
            }
        } else if let Some(config) = config {
            // TODO: can we move this to the arg parsing and surface the message more cleanly?
            assert_ne!(config.trade_rate.as_secs(), 0, "time unit must be non-zero");
            config.set(&conn)?;
        } else {
            panic!("no configuration specified")
        };

        Ok(database)
    }

    // Finally, if anything needs to talk to the database, it needs to obtain
    // a connection from the pool. Call this function with an appropriate flag.
    pub fn connect(&self, write: bool) -> Result<PooledConnection<SqliteConnectionManager>, Error> {
        let conn = if write {
            self.writer.get()
        } else {
            self.reader.get()
        };
        Ok(conn?)
    }
}

// This is just a helpful method to construct the connection pools
fn pool(
    storage: &Storage,
    max_size: Option<u32>,
    readonly: bool,
    migration: Option<Runner>,
) -> Result<Pool<SqliteConnectionManager>, Error> {
    let mut flags = OpenFlags::default();
    if readonly {
        flags.set(OpenFlags::SQLITE_OPEN_READ_WRITE, false);
        flags.set(OpenFlags::SQLITE_OPEN_READ_ONLY, true);
        flags.set(OpenFlags::SQLITE_OPEN_CREATE, false);
    }

    // Open the database
    let db = match storage {
        Storage::File(path) => SqliteConnectionManager::file(path),
        Storage::Memory(name) => {
            // for in-memory databases, SQLITE_OPEN_CREATE seems to create errors
            SqliteConnectionManager::file(format!("file:/{}?vfs=memdb", name))
        }
    }
    .with_flags(flags)
    .with_init(|c| {
        // TODO: validate these settings and possibly add to them.
        // TODO: there are only a handful of queries that we use.
        //       there might be major performance wins to cache the
        //       prepared statements here and figure out how to pass
        //       that context along.
        c.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA busy_timeout = 5000;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = true;
            PRAGMA mmap_size = 134217728;
            PRAGMA journal_size_limit = 27103364;
            PRAGMA cache_size=2000;
            "#,
        )
    });

    let pool = if let Some(n) = max_size {
        r2d2::Pool::builder().max_size(n)
    } else {
        r2d2::Pool::builder()
    }
    .build(db)?;

    if let Some(runner) = migration {
        let mut conn = pool.get()?;
        runner.run(conn.deref_mut())?;
    }

    Ok(pool)
}

// This is how you create an instance of ReaderWriter
fn open_rw(storage: Storage, migration: Option<Runner>) -> Result<Database, Error> {
    let writer = pool(&storage, Some(1), false, migration)?;
    let reader = pool(&storage, None, true, None)?;
    Ok(Database { reader, writer })
}
