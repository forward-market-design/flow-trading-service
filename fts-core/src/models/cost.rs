mod constant;
mod curve;

use crate::models::{AuthId, BidderId};
use crate::uuid_wrapper;
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

/// The data representing the "cost"
//
// TODO: Doesn't seem like the Schema is taking try_from and into into account:
// https://docs.rs/utoipa/latest/utoipa/derive.ToSchema.html#partial-serde-attributes-support
// Fixed currently with an example
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "RawCostData", into = "RawCostData")]
#[schema(example = json!([{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}]))]
pub enum CostData {
    Curve(Curve),
    Constant(Constant),
}

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("invalid demand curve: {0}")]
    Curve(#[from] curve::ValidationError),
    #[error("invalid constant curve: {0}")]
    Constraint(#[from] constant::ValidationError),
}

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

#[derive(Serialize, Deserialize, PartialEq, ToSchema, Debug)]
pub struct CostHistoryRecord {
    pub data: Option<CostData>,
    #[serde(with = "time::serde::rfc3339")]
    pub version: OffsetDateTime,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, ToSchema)]
pub struct CostRecord {
    pub bidder_id: BidderId,
    pub cost_id: CostId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<std::collections::HashMap<AuthId, f64>>)]
    pub group: Option<Group>,
    pub data: Option<CostData>,
    #[serde(with = "time::serde::rfc3339")]
    pub version: OffsetDateTime,
}

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

#[derive(Deserialize, ToSchema)]
pub struct CostDtoCreate {
    pub cost_id: Option<CostId>,
    #[schema(value_type = std::collections::HashMap<AuthId, f64>)]
    pub group: Group,
    pub data: CostData,
}

#[derive(Deserialize, ToSchema)]
pub struct CostDtoRead {
    pub cost_id: CostId,
}

#[derive(Deserialize, ToSchema)]
pub struct CostDtoUpdate {
    pub cost_id: Option<CostId>,
    pub data: CostData,
}

#[derive(Deserialize, ToSchema)]
#[serde(untagged)]
pub enum CostDto {
    Read(CostDtoRead),
    Update(CostDtoUpdate),
    Create(CostDtoCreate),
}

impl CostDto {
    pub fn cost_id(&self) -> Option<CostId> {
        match self {
            Self::Read(CostDtoRead { cost_id }) => Some(*cost_id),
            Self::Update(CostDtoUpdate { cost_id, .. }) => *cost_id,
            Self::Create(CostDtoCreate { cost_id, .. }) => *cost_id,
        }
    }
}
