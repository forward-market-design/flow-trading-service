use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

/// An outcome, whether for an auth or a product, has a price and some amount of trade.
/// Optionally, this outcome may be augmented with implementation-dependent data.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct Outcome<T> {
    pub price: f64,
    pub trade: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

/// Outcomes are usually with respect to a particular auction. This struct embeds an outcome in this context.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct AuctionOutcome<T> {
    #[serde(with = "time::serde::rfc3339")]
    pub from: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub thru: OffsetDateTime,
    #[serde(flatten)]
    pub outcome: Outcome<T>,
}
