use crate::Map;
use std::hash::Hash;

/// Solution data for an entire auction, containing the outcome for
/// each authorization and product in the market.
#[derive(Debug)]
pub struct AuctionOutcome<AuthId: Eq + Hash, ProductId: Eq + Hash> {
    /// Outcomes for each authorization, keyed by their ID
    pub auths: Map<AuthId, AuthOutcome>,
    /// Outcomes for each product, keyed by their ID
    pub products: Map<ProductId, ProductOutcome>,
    // TODO: consider a collection for the cost curves, so that we can report
    // dual information for their linear constraints
    // TODO: this struct is also a good home for market-wide summaries, such as
    // sensitivity information
}

/// Solution data for an individual authorization, containing
/// the trade quantity and effective price.
#[derive(Debug)]
pub struct AuthOutcome {
    /// The effective price for this authorization
    pub price: f64,
    /// The quantity traded for this authorization (negative for sell, positive for buy)
    pub trade: f64,
    // TODO:
    // consider reporting the dual information for the box constraint
}

/// Solution data for an individual product, containing
/// the market-clearing price and total volume traded.
#[derive(Debug)]
pub struct ProductOutcome {
    /// The market-clearing price for this product
    pub price: f64,
    /// The total quantity traded of this product (one-sided volume)
    pub volume: f64,
}
