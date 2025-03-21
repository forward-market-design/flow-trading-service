#![warn(missing_docs)]
// Note: this overwrites the link in the README to point to the rust docs of the fts-demo crate.
//! [fts_core]: https://docs.rs/fts_core/latest/fts_core/index.html
//! [fts_server]: https://docs.rs/fts_server/latest/fts_server/index.html
//! [fts_solver]: https://docs.rs/fts_solver/latest/fts_solver/index.html
//! [fts_demo]: https://docs.rs/fts_demo/latest/fts_demo/index.html
// #![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/workspace.md"))]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

/// Core domain models for the flow trading system.
///
/// This module contains the fundamental data structures that represent the domain entities.
///
/// The models in this module are primarily data structures with minimal business logic,
/// following the principles of the hexagonal architecture to separate domain entities
/// from their persistence and processing implementations.
pub mod models;

/// Interface traits for the flow trading system.
///
/// This module contains the "ports" in the hexagonal architecture pattern.
///
/// These traits define the contract between the domain logic and external adapters
/// (such as databases, APIs, or other services) without specifying implementation details.
/// This separation allows for easier testing and the ability to swap out infrastructure
/// components without affecting the core business logic.
pub mod ports;
