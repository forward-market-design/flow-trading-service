use crate::HashMap;
use std::hash::Hash;

/// The outcome of an auction
#[derive(Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(bound = "
        BidderId: Hash + Eq + serde::Serialize + serde::de::DeserializeOwned,
        PortfolioId: Clone + Hash + Eq + serde::Serialize + serde::de::DeserializeOwned,
        ProductId: Hash + Eq + Ord + serde::Serialize + serde::de::DeserializeOwned
    ")
)]
pub struct AuctionOutcome<BidderId, PortfolioId, ProductId> {
    /// The associated outcome for each submission (in turn, the portfolio trades and prices)
    pub submissions: HashMap<BidderId, HashMap<PortfolioId, PortfolioOutcome>>,

    /// The associated outcome for each traded product
    pub products: HashMap<ProductId, ProductOutcome>,
}

impl<BidderId, PortfolioId, ProductId> Default
    for AuctionOutcome<BidderId, PortfolioId, ProductId>
{
    fn default() -> Self {
        Self {
            submissions: Default::default(),
            products: Default::default(),
        }
    }
}

/// Solution data for an individual authorization, containing
/// the trade quantity and effective price.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PortfolioOutcome {
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProductOutcome {
    /// The market-clearing price for this product
    pub price: f64,
    /// The total quantity traded of this product (one-sided volume)
    pub trade: f64,
}
