use super::spvec;
use std::hash::Hash;

// A portfolio is a direction in product space
spvec!(Portfolio);

/// An auth defines a decision variable which has bounds and an associated
/// column in the market-clearing constraint.
#[derive(Debug)]
pub struct Auth<T: Eq + Hash> {
    pub min_trade: f64,
    pub max_trade: f64,
    pub portfolio: Portfolio<T>,
}
