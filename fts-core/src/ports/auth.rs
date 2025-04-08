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
    /// Returned when a bidder attempts to access or modify an auth they don't have permission for
    AccessDenied,
    /// Returned when the requested authorization does not exist in the system
    DoesNotExist,
    /// Returned when attempting to create an auth with an ID that already exists
    IdConflict,
}

/// Repository for managing authorization records and related operations.
///
/// In the flow trading system, an *auth* (short for authorization) represents:
/// 1. A *portfolio*, which is a weighted bundle of products.
/// 2. Constraints on how this portfolio can be traded.
///
/// Portfolios define what combination of products an auth trades, with weights determining
/// the relative proportions. Positive weights indicate buying the product, negative weights
/// indicate selling.
///
/// Auth constraints include:
/// - Rate constraints (min_rate, max_rate) which limit how fast a portfolio can be traded
/// - Trade constraints (min_trade, max_trade) which limit the total accumulated trade
///
/// This trait extends ProductRepository to provide functionality for creating,
/// reading, updating, and deleting authorization records, as well as querying
/// authorizations by product and retrieving historical records and auction outcomes.
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

    /// Retrieves the authorization record for the specified bidder and auth ID
    /// as of the given timestamp.
    ///
    /// Returns the authorization record if successful, or an AuthFailure if
    /// the auth does not exist or the bidder lacks access permission.
    fn read(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        as_of: OffsetDateTime,
        portfolio_options: Self::PortfolioOptions,
    ) -> impl Future<Output = Result<Result<AuthRecord, AuthFailure>, Self::Error>> + Send;

    /// Updates the data associated with an existing authorization.
    ///
    /// Returns the updated authorization record if successful, or an AuthFailure if
    /// the auth does not exist or the bidder lacks update permission.
    fn update(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        data: AuthData,
        timestamp: OffsetDateTime,
        portfolio_options: Self::PortfolioOptions,
    ) -> impl Future<Output = Result<Result<AuthRecord, AuthFailure>, Self::Error>> + Send;

    /// Marks the authorization as deleted / inactive. (This is a logical designation, not an actual removal.)
    ///
    /// Returns the deleted authorization record if successful, or an AuthFailure if
    /// the auth does not exist or the bidder lacks delete permission.
    fn delete(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        timestamp: OffsetDateTime,
        portfolio_options: Self::PortfolioOptions,
    ) -> impl Future<Output = Result<Result<AuthRecord, AuthFailure>, Self::Error>> + Send;

    /// Retrieves all active authorizations that involve the specified product as of
    /// the given timestamp for the specified bidder.
    ///
    /// Returns a vector of matching authorization records.
    fn query_by_product(
        &self,
        bidder_id: BidderId,
        product_id: ProductId,
        as_of: OffsetDateTime,
    ) -> impl Future<Output = Result<Vec<AuthRecord>, Self::Error>> + Send;

    /// Retrieves the historical records for a specific authorization within the
    /// given time range.
    ///
    /// The `limit` parameter restricts the maximum number of records returned.
    /// Returns a paginated response of auth history records if successful, or an AuthFailure if
    /// the auth does not exist or the bidder lacks access permission.
    fn get_history(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        query: DateTimeRangeQuery,
        limit: usize,
    ) -> impl Future<
        Output = Result<Result<DateTimeRangeResponse<AuthHistoryRecord>, AuthFailure>, Self::Error>,
    > + Send;

    /// Retrieves the auction outcomes associated with a specific authorization
    /// within the given time range.
    ///
    /// The `limit` parameter restricts the maximum number of outcomes returned.
    /// Returns a paginated response of auction outcomes if successful, or an AuthFailure if
    /// the auth does not exist or the bidder lacks access permission.
    fn get_outcomes(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        query: DateTimeRangeQuery,
        limit: usize,
    ) -> impl Future<
        Output = Result<Result<DateTimeRangeResponse<AuctionOutcome>, AuthFailure>, Self::Error>,
    > + Send;
}
