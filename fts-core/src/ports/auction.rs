use crate::models::{AuctionMetaData, AuthId, Outcome, ProductId, RawAuctionInput};
use crate::ports::SubmissionRepository;
use std::fmt::Debug;
use std::future::Future;
use time::{Duration, OffsetDateTime};

/// Repository trait for auction-related operations.
///
/// This trait extends [`SubmissionRepository`] to provide functionality for managing batch auctions
/// in the flow trading system. Auctions are periodic market clearing events that process all active
/// submissions to determine optimal trades and prices.
///
/// While auctions themselves are considered implementation details and not directly exposed in the
/// domain model, this trait provides methods to prepare auction inputs and report auction outcomes.
pub trait AuctionRepository: SubmissionRepository {
    /// The type used to uniquely identify auctions within the repository implementation.
    /// This allows passing auction references between preparation and reporting steps.
    ///
    /// Auctions are not directly exposed in our domain, they are an implementation detail.
    /// However, because there is a background solver thread, implementations might want to
    /// "passthru" an id between preparation and reporting steps.
    type AuctionId: Clone + Debug + Send + Sync + 'static;

    /// Create an instance of a solver for use in auction execution
    fn solver() -> impl fts_solver::Solver + Send;

    /// Gather all the required submissions for the requested auction period
    fn prepare(
        &self,
        from: Option<OffsetDateTime>,
        thru: OffsetDateTime,
        by: Option<Duration>,
        timestamp: OffsetDateTime,
    ) -> impl Future<Output = Result<Option<Vec<RawAuctionInput<Self::AuctionId>>>, Self::Error>> + Send;

    /// Store all the outcomes for the specified auction
    fn report(
        &self,
        id: Self::AuctionId,
        auth_outcomes: impl Iterator<Item = (AuthId, Outcome<()>)> + Send,
        product_outcomes: impl Iterator<Item = (ProductId, Outcome<()>)> + Send,
        timestamp: OffsetDateTime,
    ) -> impl Future<Output = Result<Option<AuctionMetaData>, Self::Error>> + Send;
}
