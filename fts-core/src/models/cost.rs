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

/// Since cost groups are immutable, it may be desired to omit their value in the response from various endpoints.
/// This type can be passed to the relevant `CostRepository` methods.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum GroupDisplay {
    Exclude,
    Include,
}

impl Default for GroupDisplay {
    fn default() -> Self {
        Self::Exclude
    }
}

/// The utility-specification of the cost
//
// TODO: Utoipa doesn't fully support all the Serde annotations,
// so we injected `untagged` (which Serde will ignore given the presence of
// of `try_from` and `into`), then inline the actual fields. This appears
// to correctly generate the OpenAPI schema, but we should revisit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(untagged, try_from = "RawCostData", into = "RawCostData")]
pub enum CostData {
    Curve(#[schema(inline)] Curve),
    Constant(#[schema(inline)] Constant),
}

/// An error type for the ways in which the provided utility function may be invalid.
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("invalid demand curve: {0}")]
    Curve(#[from] curve::ValidationError),
    #[error("invalid constant curve: {0}")]
    Constraint(#[from] constant::ValidationError),
}

/// The "DTO" type for the utility
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum RawCostData {
    Curve(Vec<Point>),
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
#[derive(Serialize, Deserialize, PartialEq, ToSchema, Debug)]
pub struct CostHistoryRecord {
    pub data: Option<CostData>,
    #[serde(with = "time::serde::rfc3339")]
    pub version: OffsetDateTime,
}

/// A full description of a cost
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
pub type Group = IndexMap<AuthId, f64, FxBuildHasher>;

impl CostRecord {
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
