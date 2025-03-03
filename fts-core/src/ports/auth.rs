use time::OffsetDateTime;

use crate::models::{
    AuctionOutcome, AuthData, AuthHistoryRecord, AuthId, AuthRecord, BidderId, DateTimeRangeQuery,
    DateTimeRangeResponse, PortfolioDisplay, ProductId,
};
use crate::ports::ProductRepository;
use std::{borrow::Borrow, future::Future};

#[derive(Debug)]
pub enum AuthFailure {
    AccessDenied,
    DoesNotExist,
    IdConflict,
}

pub trait AuthRepository: ProductRepository {
    /// Create a new authorization associated to the given account.
    fn create<K: Borrow<ProductId>, V: Borrow<f64>, P: Borrow<(K, V)>>(
        &self,
        bidder_id: BidderId,
        auth_id: Option<AuthId>,
        portfolio: impl Iterator<Item = P> + Send,
        data: AuthData,
        timestamp: OffsetDateTime,
        include_portfolio: PortfolioDisplay,
    ) -> impl Future<Output = Result<Result<AuthRecord, AuthFailure>, Self::Error>> + Send;

    /// Query for an associated authorization matching the version if specified,
    /// or the most recent authorization otherwise.
    fn read(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        as_of: OffsetDateTime,
        include_portfolio: PortfolioDisplay,
    ) -> impl Future<Output = Result<Result<AuthRecord, AuthFailure>, Self::Error>> + Send;

    /// Set the data associated to this authorization.
    fn update(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        data: AuthData,
        timestamp: OffsetDateTime,
        include_portfolio: PortfolioDisplay,
    ) -> impl Future<Output = Result<Result<AuthRecord, AuthFailure>, Self::Error>> + Send;

    /// "Delete" the authorization
    fn delete(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        timestamp: OffsetDateTime,
        include_portfolio: PortfolioDisplay,
    ) -> impl Future<Output = Result<Result<AuthRecord, AuthFailure>, Self::Error>> + Send;

    /// Finds all active auths that involve the specified product
    fn query_by_product(
        &self,
        bidder_id: BidderId,
        product_id: ProductId,
        as_of: OffsetDateTime,
    ) -> impl Future<Output = Result<Vec<AuthRecord>, Self::Error>> + Send;

    /// Retrieve the authorization history associated to this portfolio
    fn get_history(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        query: DateTimeRangeQuery,
        limit: usize,
    ) -> impl Future<
        Output = Result<Result<DateTimeRangeResponse<AuthHistoryRecord>, AuthFailure>, Self::Error>,
    > + Send;

    /// Retrieve any posted outcomes
    fn get_outcomes(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        query: DateTimeRangeQuery,
        limit: usize,
    ) -> impl Future<
        Output = Result<
            Result<DateTimeRangeResponse<AuctionOutcome<()>>, AuthFailure>,
            Self::Error,
        >,
    > + Send;
}
