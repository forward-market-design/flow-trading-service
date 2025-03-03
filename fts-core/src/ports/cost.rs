use time::OffsetDateTime;

use crate::models::{
    AuthId, BidderId, CostData, CostHistoryRecord, CostId, CostRecord, DateTimeRangeQuery,
    DateTimeRangeResponse, GroupDisplay,
};
use crate::ports::AuthRepository;
use std::{borrow::Borrow, future::Future};

#[derive(Debug)]
pub enum CostFailure {
    AccessDenied,
    DoesNotExist,
    IdConflict,
}

pub trait CostRepository: AuthRepository {
    /// Create a new cost associated to the given account.
    fn create<K: Borrow<AuthId>, V: Borrow<f64>, P: Borrow<(K, V)>>(
        &self,
        bidder_id: BidderId,
        cost_id: Option<CostId>,
        group: impl Iterator<Item = P> + Send,
        data: CostData,
        timestamp: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> impl Future<Output = Result<Result<CostRecord, CostFailure>, Self::Error>> + Send;

    /// Query for an associated bid matching the version if specified,
    /// or the most recent bid otherwise.
    fn read(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        as_of: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> impl Future<Output = Result<Result<CostRecord, CostFailure>, Self::Error>> + Send;

    /// Set the data associated to this cost.
    /// Ok(true) is a success, Ok(false) if the orderbook could not find the cost
    fn update(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        data: CostData,
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

    /// Retrieve the bid history associated to this cost
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
