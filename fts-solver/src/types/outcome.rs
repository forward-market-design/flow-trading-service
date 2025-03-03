use crate::Map;
use indexmap::Equivalent;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// Solution data for an entire auction
#[derive(Debug, Serialize, Deserialize)]
pub struct AuctionOutcome<AuthId: Eq + Hash, ProductId: Eq + Hash> {
    pub auths: Map<AuthId, AuthOutcome>,
    pub products: Map<ProductId, ProductOutcome>,
    // TODO: consider a collection for the cost curves, so that we can report
    // dual information for their linear constraints
    // TODO: this struct is also a good home for market-wide summaries, such as
    // sensitivity information
}

impl<AuthId: Eq + Hash + Ord, ProductId: Eq + Hash + Ord> AuctionOutcome<AuthId, ProductId> {
    pub fn get_auth<Q: ?Sized + Hash + Equivalent<AuthId>>(&self, key: &Q) -> Option<&AuthOutcome> {
        self.auths.get(key)
    }

    pub fn get_product<Q: ?Sized + Hash + Equivalent<ProductId>>(
        &self,
        key: &Q,
    ) -> Option<&ProductOutcome> {
        self.products.get(key)
    }

    pub fn auths(&self) -> impl Iterator<Item = (&AuthId, &AuthOutcome)> {
        self.auths.iter()
    }

    pub fn products(&self) -> impl Iterator<Item = (&ProductId, &ProductOutcome)> {
        self.products.iter()
    }
}

/// Solution data for an individual authorization
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthOutcome {
    pub price: f64,
    pub trade: f64,
    // TODO:
    // consider reporting the dual information for the box constraint
}

/// Solution data for an individual product
#[derive(Debug, Serialize, Deserialize)]
pub struct ProductOutcome {
    pub price: f64,
    pub volume: f64,
}
