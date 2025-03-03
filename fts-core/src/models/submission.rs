use crate::models::{AuthRecord, CostRecord};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct SubmissionRecord {
    pub auths: Vec<AuthRecord>,
    pub costs: Vec<CostRecord>,
    #[serde(with = "time::serde::rfc3339")]
    pub as_of: OffsetDateTime,
}
