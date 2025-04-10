use super::{AuthFailure, CostFailure};
use crate::{
    models::{AuthData, AuthId, BidderId, CostData, CostId, Group, Portfolio, SubmissionRecord},
    ports::CostRepository,
};
use serde::Deserialize;
use std::future::Future;
use time::OffsetDateTime;
use utoipa::ToSchema;

/// The various ways in which a submission may fail to process
#[derive(Debug)]
pub enum SubmissionFailure {
    /// Failure related to auth processing
    Auth(AuthFailure),
    /// Failure related to cost processing
    Cost(CostFailure),
}

/// The submission endpoint embeds a mini-CRUD interface, accordingly we need a type to embed the CRUD operations.
#[derive(Deserialize, ToSchema)]
pub struct SubmissionDto {
    /// List of auth entries for this submission
    pub auths: Vec<SubmissionAuthDto>,
    /// List of cost entries for this submission
    pub costs: Vec<SubmissionCostDto>,
}

/// For a new auth, the portfolio must be provided. To update an auth, only the data needs to be provided. To continue an existing auth as-is, only the id is required.
/// Any auths not present in the submission will be stopped.
#[derive(Deserialize, ToSchema)]
#[serde(untagged)]
pub enum SubmissionAuthDto {
    /// Create a new authorization with the specified portfolio and data
    Create {
        /// The unique identifier for the authorization
        auth_id: AuthId,
        /// The portfolio associated with this authorization
        portfolio: Portfolio,
        /// The authorization data
        data: AuthData,
    },
    /// Update an existing authorization with new data
    Update {
        /// The unique identifier for the authorization
        auth_id: AuthId,
        /// The authorization data
        data: AuthData,
    },
    /// Read an existing authorization
    Read {
        /// The unique identifier for the authorization
        auth_id: AuthId,
    },
}

impl SubmissionAuthDto {
    /// Returns the authorization ID associated with this submission
    pub fn auth_id(&self) -> AuthId {
        match self {
            Self::Create { auth_id, .. } => *auth_id,
            Self::Read { auth_id } => *auth_id,
            Self::Update { auth_id, .. } => *auth_id,
        }
    }
}

/// For a new cost, the group must be provided. To update a cost, only the data needs to be provided. To continue an existing cost as-is, only the id is required.
/// Any costs not present in the submission will be stopped.
#[derive(Deserialize, ToSchema)]
#[serde(untagged)]
pub enum SubmissionCostDto {
    /// Create a new cost with the specified group and data
    Create {
        /// The unique identifier for the cost
        cost_id: CostId,
        /// The group associated with this cost
        group: Group,
        /// The cost data
        data: CostData,
    },
    /// Update an existing cost with new data
    Update {
        /// The unique identifier for the cost
        cost_id: CostId,
        /// The cost data
        data: CostData,
    },
    /// Read an existing cost
    Read {
        /// The unique identifier for the cost
        cost_id: CostId,
    },
}

impl SubmissionCostDto {
    /// Returns the cost ID associated with this submission
    pub fn cost_id(&self) -> CostId {
        match self {
            Self::Create { cost_id, .. } => *cost_id,
            Self::Read { cost_id } => *cost_id,
            Self::Update { cost_id, .. } => *cost_id,
        }
    }
}

/// Repository trait for submission-related operations.
///
/// This trait extends [`CostRepository`] to provide functionality for managing
/// bidder submissions in the trading system. A submission represents a bidder's
/// complete set of active authorizations (auths) and costs that are considered
/// for auction processing.
pub trait SubmissionRepository: CostRepository {
    /// Get the active submission for the bidder
    fn get_submission(
        &self,
        bidder_id: BidderId,
        as_of: OffsetDateTime,
    ) -> impl Future<Output = Result<SubmissionRecord, Self::Error>> + Send;

    /// Set the current submission for the bidder, stopping any not-referenced auths or costs
    fn set_submission(
        &self,
        bidder_id: BidderId,
        submission: SubmissionDto,
        as_of: OffsetDateTime,
    ) -> impl Future<Output = Result<Result<SubmissionRecord, SubmissionFailure>, Self::Error>> + Send;
}
