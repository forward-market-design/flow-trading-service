use crate::{models::Map, ports::Repository};

/// Positions and payments are returned in integral units. The `Decimals`
/// struct specifies the scale to interpret these integers in, i.e.
/// `some_traded_amount * 10f64.powi(-decimal.trade)` gives the actual position
/// as an f64. More likely, a client would convert into a dedicated decimal
/// to work with these numbers more conveniently.
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct SettlementConfig {
    /// position * 10f64.powi(-position_decimals as i32) is actual position
    pub position_decimals: i8,
    /// payment * 10f64.powi(-payment_decimals as i32) is actual payment
    pub payment_decimals: i8,
}

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
            T::ProductId: schemars::JsonSchema,
        "
    )
)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(bound(serialize = "
            T::DateTime: serde::Serialize,
            T::BidderId: serde::Serialize,
            T::ProductId: serde::Serialize,
        "))
)]
pub struct SettlementRecord<T: Repository> {
    /// The time at which this settlement occurred
    pub as_of: T::DateTime,

    /// The id of the bidder this settlement pertains to
    pub bidder_id: T::BidderId,

    /// The decimal context for the rounded positions and payment and time unit information
    pub config: SettlementConfig,

    /// The rounded positions in this settlement
    pub positions: Map<T::ProductId, i64>,

    /// The rounded net payment in this settlement
    pub payment: i64,
}
