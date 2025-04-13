#![warn(missing_docs)]
// Note: this overwrites the link in the README to point to the rust docs of the fts-demo crate.
//! [fts_core]: https://docs.rs/fts_core/latest/fts_core/index.html
//! [fts_server]: https://docs.rs/fts_server/latest/fts_server/index.html
//! [fts_solver]: https://docs.rs/fts_solver/latest/fts_solver/index.html
//! [fts_demo]: https://docs.rs/fts_demo/latest/fts_demo/index.html
#![doc = include_str!("../README.md")]
/**
 * The various solver implementations.
 */
mod impls;
pub use impls::*;

/**
 * The core data types the solver implementations operate on.
 */
mod types;
pub use types::*;

// For reproducibility, we need explicitly ordered semantics in our collections.
// Accordingly, we swap out the stdlib collections for those provided by `indexmap`.
// Since we're swapping out these types already, we can benefit from a hash function
// that is optimized for small collections.
pub(crate) type HashMap<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;
pub(crate) type HashSet<T> = indexmap::IndexSet<T, rustc_hash::FxBuildHasher>;
