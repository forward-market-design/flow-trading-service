/// Solution data for an individual portfolio, containing
/// the trade rate and effective price.
#[derive(Debug)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PortfolioOutcome {
    /// The effective price for this portfolio
    pub price: f64,
    /// The rate of trade of this portfolio (negative for sell, positive for buy)
    pub rate: f64,
    // TODO:
    // consider reporting the dual information for the box constraint
}

impl Default for PortfolioOutcome {
    fn default() -> Self {
        Self {
            price: f64::NAN,
            rate: 0.0,
        }
    }
}

/// Solution data for an individual product, containing
/// the market-clearing price and total volume traded.
#[derive(Debug)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProductOutcome {
    /// The market-clearing price for this product
    pub price: f64,
    /// The rate of trade of this product
    pub rate: f64,
}

impl Default for ProductOutcome {
    fn default() -> Self {
        Self {
            price: f64::NAN,
            rate: 0.0,
        }
    }
}
