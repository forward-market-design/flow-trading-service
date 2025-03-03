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
