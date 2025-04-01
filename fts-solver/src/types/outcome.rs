use serde::Serialize;

use crate::Map;
use std::hash::Hash;

/// Solution data for an entire auction, containing the outcome for
/// each authorization and product in the market.
#[derive(Debug)]
pub struct AuctionOutcome<BidderId: Eq + Hash, AuthId: Eq + Hash, ProductId: Eq + Hash> {
    /// Outcomes for each submission
    pub outcomes: Map<BidderId, SubmissionOutcome<AuthId>>,
    /// Outcomes for each product, keyed by their ID
    pub products: Map<ProductId, ProductOutcome>,
    // TODO: consider a collection for the cost curves, so that we can report
    // dual information for their linear constraints
    // TODO: this struct is also a good home for market-wide summaries, such as
    // sensitivity information
}

impl<BidderId: Eq + Hash, AuthId: Eq + Hash, ProductId: Eq + Hash> Default
    for AuctionOutcome<BidderId, AuthId, ProductId>
{
    fn default() -> Self {
        Self {
            outcomes: Default::default(),
            products: Default::default(),
        }
    }
}

/// Gather all the outcomes pertaining to a submission.
#[derive(Debug, Serialize)]
pub struct SubmissionOutcome<AuthId: Eq + Hash> {
    /// The mapping of auths to their outcomes
    pub auths: Map<AuthId, AuthOutcome>,
}

impl<AuthId: Eq + Hash> Default for SubmissionOutcome<AuthId> {
    fn default() -> Self {
        Self {
            auths: Default::default(),
        }
    }
}

/// Solution data for an individual authorization, containing
/// the trade quantity and effective price.
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
pub struct ProductOutcome {
    /// The market-clearing price for this product
    pub price: f64,
    /// The total quantity traded of this product (one-sided volume)
    pub trade: f64,
}
