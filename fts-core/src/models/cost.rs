use crate::models::{AuthId, BidderId, DemandCurve, map_wrapper, uuid_wrapper};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
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

/// A record of the cost's data at the time it was updated or defined
///
/// This provides historical versioning of cost data, allowing the system
/// to track changes to cost definitions over time.
#[derive(Serialize, Deserialize, PartialEq, ToSchema, Debug)]
pub struct CostHistoryRecord {
    /// The cost data, or None if the cost was deactivated
    pub data: Option<DemandCurve>,
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
    pub data: Option<DemandCurve>,

    /// The "last-modified-or-created" time as recorded by the system
    #[serde(with = "time::serde::rfc3339")]
    pub version: OffsetDateTime,
}

map_wrapper!(Group, AuthId, f64);

impl CostRecord {
    /// Converts this cost record into a solver-compatible format.
    ///
    /// This method transforms the cost record into the appropriate solver structures,
    /// applying the time scale to rate-based constraints as needed.
    pub fn into_solver(
        self,
        scale: f64,
    ) -> Option<fts_solver::DemandCurve<AuthId, Group, Vec<fts_solver::Point>>> {
        if let Some(data) = self.data {
            let group = self.group.unwrap_or_default();
            let points = match data {
                DemandCurve::Curve(curve) => curve.as_solver(scale),
                DemandCurve::Constant(constant) => constant.as_solver(scale),
            };
            let domain = (
                points.first().map(|pt| pt.quantity).unwrap_or_default(),
                points.last().map(|pt| pt.quantity).unwrap_or_default(),
            );

            Some(fts_solver::DemandCurve {
                domain,
                group,
                points,
            })
        } else {
            None
        }
    }
}
