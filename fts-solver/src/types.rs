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

/// All solvers must adhere to the folllowing interface
pub trait Solver {
    type Settings;

    /// Create a new instance with the provided settings
    fn new(settings: Self::Settings) -> Self;

    /// Construct and solve the corresponding quadratic program
    fn solve<
        AuthId: Eq + std::hash::Hash + Clone + Ord,
        ProductId: Eq + std::hash::Hash + Clone + Ord,
    >(
        &self,
        auction: &[Submission<AuthId, ProductId>],
        // TODO: warm-starts with the prices
    ) -> AuctionOutcome<AuthId, ProductId>;
}
