use crate::{
    models::{
        AuctionOutcome, AuthData, AuthHistoryRecord, AuthId, AuthRecord, BidderId,
        DateTimeRangeQuery, DateTimeRangeResponse, ProductId,
    },
    ports::ProductRepository,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::{borrow::Borrow, future::Future};
use time::OffsetDateTime;

#[derive(Debug)]
pub enum AuthFailure {
    AccessDenied,
    DoesNotExist,
    IdConflict,
}

pub trait AuthRepository: ProductRepository {
    /// Implementations may elect to display (or not) the porfolio in various
    /// ways. At a minimum, this should be a bool on whether or not to include
    /// the portfolio in a response; in a forward market application with
    /// increasingly granular products, this might be an option to display the
    /// original portfolio or the "effective" portfolio.
    type PortfolioOptions: Default + Serialize + DeserializeOwned + Send;

    /// Create a new authorization associated to the given account.
    fn create<K: Borrow<ProductId>, V: Borrow<f64>, P: Borrow<(K, V)>>(
        &self,
        bidder_id: BidderId,
        auth_id: Option<AuthId>,
        portfolio: impl Iterator<Item = P> + Send,
        data: AuthData,
        timestamp: OffsetDateTime,
        portfolio_options: Self::PortfolioOptions,
    ) -> impl Future<Output = Result<Result<AuthRecord, AuthFailure>, Self::Error>> + Send;

    /// Query for an associated authorization matching the version if specified,
    /// or the most recent authorization otherwise.
    fn read(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        as_of: OffsetDateTime,
        portfolio_options: Self::PortfolioOptions,
    ) -> impl Future<Output = Result<Result<AuthRecord, AuthFailure>, Self::Error>> + Send;

    /// Set the data associated to this authorization.
    fn update(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        data: AuthData,
        timestamp: OffsetDateTime,
        portfolio_options: Self::PortfolioOptions,
    ) -> impl Future<Output = Result<Result<AuthRecord, AuthFailure>, Self::Error>> + Send;

    /// "Delete" the authorization
    fn delete(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        timestamp: OffsetDateTime,
        portfolio_options: Self::PortfolioOptions,
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
