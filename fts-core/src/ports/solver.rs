use crate::models::{DemandCurve, DemandGroup, Map, Basis};
use std::hash::Hash;

/// Interface for optimization solvers that compute market clearing solutions.
///
/// A solver takes demand curves and portfolio configurations as input and
/// produces optimal allocations and prices.
pub trait Solver<DemandId: Eq + Hash, PortfolioId: Eq + Hash, ProductId: Eq + Hash> {
    /// Error type for solver failures
    type Error: std::error::Error;

    /// A solver will produce outcomes for a portfolio. This should, at a
    /// minimum, include the optimal trade rate. It might also include the
    /// effective portfolio price, as well as other aggregated statistics.
    type PortfolioOutcome;

    /// A solver will produce outcomes for a product. This should, at a
    /// minimum, include the clearing price. It might also include the total
    /// speed of trade, as well as other aggregated statistics.
    type ProductOutcome;

    /// A solver may leverage additional data to generate a solution, such as
    /// a previous batch's prices and trades.
    ///
    /// The Default implementation should provide a reasonable initial state.
    type State: Default;

    /// Produce a solution given the batch inputs and the solver state.
    ///
    /// # Arguments
    ///
    /// - `demand_curves`: Map of demand IDs to their curves
    /// - `portfolios`: Map of portfolio IDs to their (demand weights, product weights)
    /// - `state`: Previous solver state for warm-starting or continuity
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// - Portfolio outcomes mapping portfolio IDs to their allocations
    /// - Product outcomes mapping product IDs to their clearing prices
    fn solve(
        &self,
        demand_curves: Map<DemandId, DemandCurve>,
        portfolios: Map<PortfolioId, (DemandGroup<DemandId>, Basis<ProductId>)>,
        state: Self::State,
    ) -> impl Future<
        Output = Result<
            (
                Map<PortfolioId, Self::PortfolioOutcome>,
                Map<ProductId, Self::ProductOutcome>,
            ),
            Self::Error,
        >,
    > + Send;
}
