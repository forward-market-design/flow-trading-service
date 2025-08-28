use crate::models::{Basis, DemandCurve, Map, Weights};
use std::hash::Hash;

/// A generic outcome for tradeable things.
///
/// At a minimum, it must include a semantically-relevant trade amount and optional price
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema), schemars(inline))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Outcome<Data> {
    /// The amount traded of the underyling thing. Must be finite and non-nan.
    pub trade: f64,

    /// The price of the underlying thing. Must be finite and non-nan, or None.
    pub price: Option<f64>,

    /// Additional data to attach to the outcome, such as an aggregate demand curve or other statistic.
    pub data: Data,
}

impl<Data: Default> Default for Outcome<Data> {
    fn default() -> Self {
        Self {
            trade: 0.0,
            price: None,
            data: Default::default(),
        }
    }
}

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
        portfolios: Map<PortfolioId, (Weights<DemandId>, Basis<ProductId>)>,
        state: Self::State,
    ) -> impl Future<
        Output = Result<
            (
                Map<PortfolioId, Outcome<Self::PortfolioOutcome>>,
                Map<ProductId, Outcome<Self::ProductOutcome>>,
            ),
            Self::Error,
        >,
    > + Send;
}
