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

/// AuthRepository methods are expected to enforce various restrictions on user access.
/// In particular, if a client-generated ID conflicts with one already present in the system,
/// an error must be returned. If a bidder tries to obtain information on a different bidder's
/// auth, this action must fail.
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

    /// Create a new authorization associated to the given bidder.
    ///
    /// If `auth_id` is None, assigns a system-generated ID.
    fn create<K: Borrow<ProductId>, V: Borrow<f64>, P: Borrow<(K, V)>>(
        &self,
        bidder_id: BidderId,
        auth_id: Option<AuthId>,
        portfolio: impl Iterator<Item = P> + Send,
        data: AuthData,
        timestamp: OffsetDateTime,
        portfolio_options: Self::PortfolioOptions,
    ) -> impl Future<Output = Result<Result<AuthRecord, AuthFailure>, Self::Error>> + Send;

    /// Get the record for the requested auth as of the specified time
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

    /// Retrieve the history associated to the auth
    fn get_history(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        query: DateTimeRangeQuery,
        limit: usize,
    ) -> impl Future<
        Output = Result<Result<DateTimeRangeResponse<AuthHistoryRecord>, AuthFailure>, Self::Error>,
    > + Send;

    /// Retrieve any posted outcomes for the auth
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
