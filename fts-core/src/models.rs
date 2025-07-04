//! Core domain models for the flow trading system.
//!
//! This module contains the fundamental data structures that represent the domain entities.
//!
//! The models in this module are primarily data structures with minimal business logic,
//! following the principles of the hexagonal architecture to separate domain entities
//! from their persistence and processing implementations.

mod curve;
pub use curve::*;

mod portfolio;
pub use portfolio::*;

mod demand;
pub use demand::*;

mod product;
pub use product::*;

mod map;
pub use map::*;

mod datetime;
pub use datetime::*;

mod group;
pub use group::*;

/// A timestamped record of a component of a user's bid.
///
/// The interval for which the component has this value is provided alongside
/// the value itself.
#[cfg_attr(
    feature = "schemars",
    derive(schemars::JsonSchema),
    schemars(rename = "{Value}Record")
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValueRecord<DateTime, Value> {
    /// The timestamp when this change occurred
    pub valid_from: DateTime,
    /// The timestamp when this change was superceded
    pub valid_until: Option<DateTime>,
    /// The component value
    pub value: Value,
}
