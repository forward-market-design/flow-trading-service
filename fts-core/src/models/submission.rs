use crate::models::{AuthRecord, CostRecord};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

/// A submission is an intermediate object of current auths and costs, not a first-class primitive.
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
