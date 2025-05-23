#![warn(missing_docs)]
// Note: this overwrites the link in the README to point to the rust docs of the fts-sqlite crate.
//! [fts_core]: https://docs.rs/fts_core/latest/fts_core/index.html
//! [fts_server]: https://docs.rs/fts_server/latest/fts_server/index.html
//! [fts_solver]: https://docs.rs/fts_solver/latest/fts_solver/index.html
//! [fts_sqlite]: https://docs.rs/fts_sqlite/latest/fts_sqlite/index.html
#![doc = include_str!("../README.md")]

/// Database operations and connection management
pub mod db;
mod impls;

// This manages our database setup/migrations
mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./sql");
}

mod datetime;
pub use datetime::DateTime;
