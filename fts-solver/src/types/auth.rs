use super::spvec;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

// A portfolio is a direction in product space
spvec!(Portfolio);

// We will associate bounds to a portfolio as part of an authorization
#[derive(Debug, Serialize, Deserialize)]
pub struct Auth<T: Eq + Hash> {
    pub min_trade: f64,
    pub max_trade: f64,
    pub portfolio: Portfolio<T>,
}
