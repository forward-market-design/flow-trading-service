use time::OffsetDateTime;

use crate::models::{
    AuthId, BidderId, CostHistoryRecord, CostId, CostRecord, DateTimeRangeQuery,
    DateTimeRangeResponse, DemandCurve, GroupDisplay,
};
use crate::ports::AuthRepository;
use std::{borrow::Borrow, future::Future};

/// CostRepository methods are expected to enforce various restrictions on user access.
/// In particular, if a client-generated ID conflicts with one already present in the system,
/// an error must be returned. If a bidder tries to obtain information on a different bidder's
/// auth, this action must fail.
#[derive(Debug)]
pub enum CostFailure {
    /// The requester does not have permission to access the resource
    AccessDenied,
    /// The requested cost does not exist
    DoesNotExist,
    /// The provided ID conflicts with an existing cost ID
    IdConflict,
}

/// Repository for managing cost records
///
/// In the flow trading system, a cost represents two things:
/// 1. A definition of a cost *group*, which is a linear combination of auths
/// 2. A piecewise-linear or constant demand *curve*
///
/// Cost groups support expressing substitution preferences between portfolios.
/// For example, if products A and B are perfect substitutes, a cost group could
/// include auths for both products with equal weights.
///
/// The demand curve represents the bidder's willingness to pay for different
/// quantities of the group's combined trade.
///
/// This trait provides methods for creating, reading, updating, and deleting cost
/// records, as well as retrieving cost history.
pub trait CostRepository: AuthRepository {
    /// Create a new cost associated to the given bidder.
    ///
    /// If `cost_id` is None, assigns a system-generated ID.
    fn create<K: Borrow<AuthId>, V: Borrow<f64>, P: Borrow<(K, V)>>(
        &self,
        bidder_id: BidderId,
        cost_id: Option<CostId>,
        group: impl Iterator<Item = P> + Send,
        data: DemandCurve,
        timestamp: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> impl Future<Output = Result<Result<CostRecord, CostFailure>, Self::Error>> + Send;

    /// Get the record for the requested cost as of the specified time
    fn read(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        as_of: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> impl Future<Output = Result<Result<CostRecord, CostFailure>, Self::Error>> + Send;

    /// Set the data associated to this cost
    fn update(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        data: DemandCurve,
        timestamp: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> impl Future<Output = Result<Result<CostRecord, CostFailure>, Self::Error>> + Send;

    /// "Delete" the cost
    fn delete(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        timestamp: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> impl Future<Output = Result<Result<CostRecord, CostFailure>, Self::Error>> + Send;

    // /// Finds all active costs that reference the specified auth
    // fn query_by_auth(
    //     &self,
    //     bidder_id: &BidderId,
    //     auth_id: &AuthId,
    //     as_of: &OffsetDateTime,
    // ) -> impl Future<Output = Result<Vec<CostRecord>, Self::Error>> + Send;

    // /// Finds all active costs that involve the specified product
    // fn query_by_product(
    //     &self,
    //     bidder_id: &BidderId,
    //     product_id: &ProductId,
    //     as_of: &DateTime,
    // ) -> impl Future<Output = Result<Vec<CostRecord>, Self::Error>> + Send;

    /// Retrieve the history associated to this cost
    fn get_history(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        query: DateTimeRangeQuery,
        limit: usize,
    ) -> impl Future<
        Output = Result<Result<DateTimeRangeResponse<CostHistoryRecord>, CostFailure>, Self::Error>,
    > + Send;
}
