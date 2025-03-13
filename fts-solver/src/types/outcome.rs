use crate::Map;
use std::hash::Hash;

/// Solution data for an entire auction
#[derive(Debug)]
pub struct AuctionOutcome<AuthId: Eq + Hash, ProductId: Eq + Hash> {
    pub auths: Map<AuthId, AuthOutcome>,
    pub products: Map<ProductId, ProductOutcome>,
    // TODO: consider a collection for the cost curves, so that we can report
    // dual information for their linear constraints
    // TODO: this struct is also a good home for market-wide summaries, such as
    // sensitivity information
}

/// Solution data for an individual authorization
#[derive(Debug)]
pub struct AuthOutcome {
    pub price: f64,
    pub trade: f64,
    // TODO:
    // consider reporting the dual information for the box constraint
}

/// Solution data for an individual product
#[derive(Debug)]
pub struct ProductOutcome {
    pub price: f64,
    pub volume: f64,
}
