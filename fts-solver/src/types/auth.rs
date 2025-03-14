use super::spvec;
use std::hash::Hash;

// A portfolio is a direction in product space
spvec!(Portfolio);

/// An authorization defines a decision variable for trading.
/// It specifies the allowable trade range and the portfolio composition.
///
/// The min_trade and max_trade values define bounds on how much of the portfolio
/// can be traded. For example:
/// - For buy-only: min_trade = 0.0, max_trade > 0.0
/// - For sell-only: min_trade < 0.0, max_trade = 0.0
/// - For two-sided: min_trade < 0.0, max_trade > 0.0
#[derive(Debug)]
pub struct Auth<T: Eq + Hash> {
    /// Minimum allowable trade quantity (typically <= 0)
    pub min_trade: f64,
    /// Maximum allowable trade quantity (typically >= 0)
    pub max_trade: f64,
    /// The composition of the portfolio being traded
    pub portfolio: Portfolio<T>,
}
