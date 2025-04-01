use std::hash::Hash;

mod submission;
pub use submission::*;

mod spvec;
pub(crate) use spvec::spvec;

mod auth;
pub use auth::*;

mod cost;
pub use cost::*;

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
        AuthId: Eq + Hash + Clone + Ord,
        ProductId: Eq + Hash + Clone + Ord,
    >(
        &self,
        auction: &T,
        // TODO: warm-starts with the prices
    ) -> AuctionOutcome<BidderId, AuthId, ProductId>
    where
        for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<AuthId, ProductId>)>;
}
