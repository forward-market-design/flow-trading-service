use crate::ports::Repository;

/// Positions and payments are returned in integral units. The `Decimals`
/// struct specifies the scale to interpret these integers in, i.e.
/// `some_traded_amount * 10f64.powi(-decimal.trade)` gives the actual position
/// as an f64. More likely, a client would convert into a dedicated decimal
/// to work with these numbers more conveniently.
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SettlementConfig {
    /// position * 10f64.powi(-position_decimals as i32) is actual position
    pub position_decimals: i8,
    /// payment * 10f64.powi(-payment_decimals as i32) is actual payment
    pub payment_decimals: i8,
}

impl SettlementConfig {
    fn round(values: &[f64], scale: i8) -> Vec<Amount> {
        let scale = 10f64.powi(scale as i32);

        let test: Vec<(u64, u64, f64)> = values
            .into_iter()
            .map(|x| {
                let units = x * scale;
                let whole = units.trunc();
                let error = units - whole;
                let whole = whole as i64;
                let sell = 0i64.max(-whole) as u64;
                let buy = 0i64.max(whole) as u64;

                (sell, buy, error)
            })
            .collect();

        todo!()
    }

    /// Given a slice of signed positions, round in a signed, sum-preserving manner.
    pub fn round_positions(&self, values: &[f64]) -> Vec<Amount> {
        Self::round(values, self.position_decimals)
    }

    /// Given a slice of signed payments, round in a signed, sum-preserving manner.
    pub fn round_payments(&self, values: &[f64]) -> Vec<Amount> {
        Self::round(values, self.payment_decimals)
    }
}

/// A basic newtype to represent a cumulative amount. Instances of this struct
/// should be "inert" without an accompanying `Decimal` context.
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema), schemars(inline))]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(transparent))]
pub struct Amount(pub i64);

/// Represents a period of settled activity for a bidder.
///
/// Note that, elsewhere, "trade" is understood to be a rate. Settlement
/// integrates these rates over time, accumulating each into a "position".
#[cfg_attr(
    feature = "schemars",
    derive(schemars::JsonSchema),
    schemars(
        rename = "SettlementRecord",
        bound = "
            T::DateTime: schemars::JsonSchema,
            T::BidderId: schemars::JsonSchema,
            ProductMap: schemars::JsonSchema,
        "
    )
)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(bound(serialize = "
            T::DateTime: serde::Serialize,
            T::BidderId: serde::Serialize,
            ProductMap: serde::Serialize,
        "))
)]
pub struct SettlementRecord<T: Repository, ProductMap: FromIterator<(T::ProductId, Amount)>> {
    /// The time at which this settlement occurred
    pub as_of: T::DateTime,

    /// The id of the bidder this settlement pertains to
    pub bidder_id: T::BidderId,

    /// The decimal context for the rounded positions and payment and time unit information
    pub config: SettlementConfig,

    /// The rounded positions in this settlement
    pub positions: ProductMap,

    /// The rounded net payment in this settlement
    pub payment: Amount,
}
