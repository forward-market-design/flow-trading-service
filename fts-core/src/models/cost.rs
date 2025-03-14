mod constant;
mod curve;

use crate::models::{AuthId, BidderId, uuid_wrapper};
pub use constant::{Constant, RawConstant};
pub use curve::{Curve, Point};
use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use thiserror::Error;
use time::OffsetDateTime;
use utoipa::ToSchema;

// A simple newtype for a Uuid
uuid_wrapper!(CostId);

/// Controls whether cost group details should be included in API responses
///
/// Since cost groups can be large, this enum allows API endpoints to optionally
/// exclude group details from responses to reduce payload size.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum GroupDisplay {
    /// Exclude group details from the response
    Exclude,
    /// Include group details in the response
    Include,
}

impl Default for GroupDisplay {
    fn default() -> Self {
        Self::Exclude
    }
}

/// The utility-specification of the cost
///
/// CostData represents either:
/// - A non-increasing, piecewise-linear demand curve assigning a cost to each quantity in its domain, or
/// - A simple, "flat" demand curve assining a constant cost to each quantity in its domain.
///
/// This is the core component that defines how a bidder values different trade outcomes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(untagged, try_from = "RawCostData", into = "RawCostData")]
// TODO: Utoipa doesn't fully support all the Serde annotations,
// so we injected `untagged` (which Serde will ignore given the presence of
// of `try_from` and `into`), then inline the actual fields. This appears
// to correctly generate the OpenAPI schema, but we should revisit.
pub enum CostData {
    /// A piecewise linear demand curve defined by points
    Curve(#[schema(inline)] Curve),
    /// A constant constraint enforcing a specific trade quantity at a price
    Constant(#[schema(inline)] Constant),
}

/// An error type for the ways in which the provided utility function may be invalid.
#[derive(Error, Debug)]
pub enum ValidationError {
    /// Error when a curve's definition is invalid
    #[error("invalid demand curve: {0}")]
    Curve(#[from] curve::ValidationError),
    /// Error when a constant curve's definition is invalid
    #[error("invalid constant curve: {0}")]
    Constraint(#[from] constant::ValidationError),
}

/// The "DTO" type for the utility
///
/// This enum represents the raw data formats accepted in API requests for defining costs.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum RawCostData {
    /// A sequence of points defining a piecewise linear demand curve
    Curve(Vec<Point>),
    /// A raw constant constraint definition
    Constant(RawConstant),
}

impl TryFrom<RawCostData> for CostData {
    type Error = ValidationError;

    fn try_from(value: RawCostData) -> Result<Self, Self::Error> {
        match value {
            RawCostData::Curve(curve) => Ok(CostData::Curve(curve.try_into()?)),
            RawCostData::Constant(constant) => Ok(CostData::Constant(constant.try_into()?)),
        }
    }
}

impl From<CostData> for RawCostData {
    fn from(value: CostData) -> Self {
        match value {
            CostData::Curve(curve) => RawCostData::Curve(curve.into()),
            CostData::Constant(constant) => RawCostData::Constant(constant.into()),
        }
    }
}

/// A record of the cost's data at the time it was updated or defined
///
/// This provides historical versioning of cost data, allowing the system
/// to track changes to cost definitions over time.
#[derive(Serialize, Deserialize, PartialEq, ToSchema, Debug)]
pub struct CostHistoryRecord {
    /// The cost data, or None if the cost was deactivated
    pub data: Option<CostData>,
    /// The timestamp when this version was created
    #[serde(with = "time::serde::rfc3339")]
    pub version: OffsetDateTime,
}

/// A full description of a cost
///
/// A CostRecord combines all the information needed to define a cost:
/// - Who owns it (bidder_id)
/// - Which auths it applies to (group)
/// - The utility function (data)
#[derive(Serialize, Deserialize, PartialEq, Debug, ToSchema)]
pub struct CostRecord {
    /// The responsible bidder's id
    pub bidder_id: BidderId,

    /// A unique id for the cost
    pub cost_id: CostId,

    /// The group associated to the cost. Because it is not always required, some endpoints may omit its definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<std::collections::HashMap<AuthId, f64>>)]
    pub group: Option<Group>,

    /// The utility for the cost
    pub data: Option<CostData>,

    /// The "last-modified-or-created" time as recorded by the system
    #[serde(with = "time::serde::rfc3339")]
    pub version: OffsetDateTime,
}

/// A group is a sparse collection of authorizations
///
/// Groups define which auths a particular cost applies to, with weights determining the
/// relative contribution of each auth to the group's overall trade. This allows bidders
/// to express substitution preferences between different portfolios.
pub type Group = IndexMap<AuthId, f64, FxBuildHasher>;

impl CostRecord {
    /// Converts this cost record into a solver-compatible format.
    ///
    /// This method transforms the cost record into the appropriate solver structures,
    /// applying the time scale to rate-based constraints as needed.
    pub fn into_solver(self, scale: f64) -> Option<(fts_solver::Group<AuthId>, fts_solver::Cost)> {
        let group = self.group.unwrap_or_default().into_iter().collect();

        if let Some(data) = self.data {
            Some((
                group,
                match data {
                    CostData::Curve(curve) => {
                        let curve = curve.as_solver(scale);
                        if curve.points.len() == 1 {
                            // We can convert a pathological curve into a constraint
                            let point = curve.points.first().unwrap();
                            // assert_eq!(point.quantity, 0.0);
                            let constant = fts_solver::Constant {
                                quantity: (point.quantity, point.quantity),
                                price: point.price,
                            };
                            fts_solver::Cost::Constant(constant)
                        } else {
                            fts_solver::Cost::PiecewiseLinearCurve(curve)
                        }
                    }
                    CostData::Constant(constant) => {
                        fts_solver::Cost::Constant(constant.as_solver(scale))
                    }
                },
            ))
        } else {
            None
        }
    }
}
