#![warn(missing_docs)]
// Note: this overwrites the link in the README to point to the rust docs of the fts-demo crate.
//! [fts_core]: https://docs.rs/fts_core/latest/fts_core/index.html
//! [fts_server]: https://docs.rs/fts_server/latest/fts_server/index.html
//! [fts_solver]: https://docs.rs/fts_solver/latest/fts_solver/index.html
//! [fts_demo]: https://docs.rs/fts_demo/latest/fts_demo/index.html
#![doc = include_str!("../../docs/workspace.md")]
#![doc = include_str!("../README.md")]
/**
 * These are implementations of the flow trading solver.
 */
mod impls;
pub use impls::*;

/**
 * These are the core data types the implementations operate on.
 */
mod types;
pub use types::*;

// We use non-std collections here for their ordering semantics and performance
pub(crate) type Map<K, V> = indexmap::IndexMap<K, V, fxhash::FxBuildHasher>;
pub(crate) type Set<T> = indexmap::IndexSet<T, fxhash::FxBuildHasher>;
