use std::fmt::Debug;
use std::hash::Hash;

mod submission;
pub use submission::*;

mod demand;
pub use demand::*;

mod outcome;
pub use outcome::*;

/// The Solver trait defines the interface for market-clearing solvers.
///
/// A Solver takes market participant submissions (bids/offers) and computes
/// the optimal trades and market-clearing prices that maximize total welfare.
///
/// Implementations may use different optimization algorithms with varying
/// performance and precision characteristics.
pub trait Solver {
    /// The configuration type for this solver
    type Settings;

    /// The status type (returned as Err when not successful)
    type Status: Debug;

    /// Create a new instance with the provided settings
    fn new(settings: Self::Settings) -> Self;

    /// Solve the market clearing problem for the given auction submissions
    ///
    /// # Parameters
    /// * `auction` - A slice of Submission objects representing all bids and offers
    ///
    /// # Returns
    /// * `AuctionOutcome` - Contains the clearing prices and trades for each product and authorization
    fn solve<
        T,
        BidderId: Eq + Hash + Clone + Ord,
        PortfolioId: Eq + Hash + Clone + Ord,
        ProductId: Eq + Hash + Clone + Ord,
    >(
        &self,
        auction: &T,
        // TODO: warm-starts with the prices
    ) -> Result<AuctionOutcome<BidderId, PortfolioId, ProductId>, Self::Status>
    where
        for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<PortfolioId, ProductId>)>;
}
