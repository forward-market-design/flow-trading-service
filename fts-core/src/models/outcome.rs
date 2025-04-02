use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

/// An outcome represents the result of an auction for a particular auth or product.
///
/// An outcome contains:
/// - A `price` at which the asset was cleared in the auction
/// - A `trade` amount representing the quantity that was traded
/// - Optional implementation-dependent `data` for additional context
///
/// The sign convention for trades follows the flow trading standard:
/// - Positive values indicate buying
/// - Negative values indicate selling
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct Outcome<T> {
    /// The clearing price determined by the auction solver
    #[serde(with = "safe_f64")]
    pub price: f64,
    /// The trade amount (positive for buying, negative for selling)
    pub trade: f64,
    /// Optional implementation-specific data related to the outcome
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

/// Provides temporal context for an outcome by associating it with
/// a specific auction time interval.
///
/// This structure embeds an `Outcome` with the time range (`from` and `thru`)
/// of the auction that produced it, allowing for tracking outcomes across
/// multiple consecutive auctions.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct AuctionOutcome<T> {
    /// The starting time of the auction interval
    #[serde(with = "time::serde::rfc3339")]
    pub from: OffsetDateTime,
    /// The ending time of the auction interval
    #[serde(with = "time::serde::rfc3339")]
    pub thru: OffsetDateTime,
    /// The actual outcome (price and trade) from the auction
    #[serde(flatten)]
    pub outcome: Outcome<T>,
}

mod safe_f64 {
    use serde::{Deserialize as _, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        if value.is_finite() {
            serializer.serialize_some(value)
        } else {
            serializer.serialize_none()
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        Ok(Option::<f64>::deserialize(deserializer)?.unwrap_or(f64::NAN))
    }
}
