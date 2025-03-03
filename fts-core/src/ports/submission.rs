use super::{AuthFailure, CostFailure};
use crate::{
    models::{AuthData, AuthId, BidderId, CostData, CostId, Group, Portfolio, SubmissionRecord},
    ports::CostRepository,
};
use serde::Deserialize;
use std::future::Future;
use time::OffsetDateTime;
use utoipa::ToSchema;

pub trait SubmissionRepository: CostRepository {
    /// Get the active submission for the bidder
    fn get_submission(
        &self,
        bidder_id: BidderId,
        as_of: OffsetDateTime,
    ) -> impl Future<Output = Result<SubmissionRecord, Self::Error>> + Send;

    fn set_submission(
        &self,
        bidder_id: BidderId,
        submission: SubmissionDto,
        as_of: OffsetDateTime,
    ) -> impl Future<Output = Result<Result<SubmissionRecord, SubmissionFailure>, Self::Error>> + Send;
}

#[derive(Deserialize, ToSchema)]
pub struct SubmissionDto {
    pub auths: Vec<SubmissionAuthDto>,
    pub costs: Vec<SubmissionCostDto>,
}

#[derive(Deserialize, ToSchema)]
#[serde(untagged)]
pub enum SubmissionAuthDto {
    Create {
        auth_id: AuthId,
        #[schema(value_type = std::collections::HashMap<crate::models::ProductId, f64>)]
        portfolio: Portfolio,
        data: AuthData,
    },
    Update {
        auth_id: AuthId,
        data: AuthData,
    },
    Read {
        auth_id: AuthId,
    },
}

impl SubmissionAuthDto {
    pub fn auth_id(&self) -> AuthId {
        match self {
            Self::Create { auth_id, .. } => *auth_id,
            Self::Read { auth_id } => *auth_id,
            Self::Update { auth_id, .. } => *auth_id,
        }
    }
}

#[derive(Deserialize, ToSchema)]
#[serde(untagged)]
pub enum SubmissionCostDto {
    Create {
        cost_id: CostId,
        #[schema(value_type = std::collections::HashMap<crate::models::AuthId, f64>)]
        group: Group,
        data: CostData,
    },
    Update {
        cost_id: CostId,
        data: CostData,
    },
    Read {
        cost_id: CostId,
    },
}

impl SubmissionCostDto {
    pub fn cost_id(&self) -> CostId {
        match self {
            Self::Create { cost_id, .. } => *cost_id,
            Self::Read { cost_id } => *cost_id,
            Self::Update { cost_id, .. } => *cost_id,
        }
    }
}

#[derive(Debug)]
pub enum SubmissionFailure {
    Auth(AuthFailure),
    Cost(CostFailure),
}
