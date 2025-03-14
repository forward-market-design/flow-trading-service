use crate::models::{AuthRecord, CostRecord};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

/// A submission represents a bidder's complete set of active authorizations and costs
/// that are considered for auction processing.
///
/// Submissions are not first-class primitives, but intermediate objects and not persisted entities.
/// They're constructed from the current state of a bidder's auths and costs when needed for an auction.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct SubmissionRecord {
    /// A list of "active" auths
    pub auths: Vec<AuthRecord>,

    /// A list of "active" costs
    pub costs: Vec<CostRecord>,

    /// The system-time at which these lists were generated
    #[serde(with = "time::serde::rfc3339")]
    pub as_of: OffsetDateTime,
}
