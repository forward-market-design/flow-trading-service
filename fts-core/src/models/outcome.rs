use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct Outcome<T: ToSchema> {
    pub price: f64,
    pub trade: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct AuctionOutcome<T: ToSchema> {
    #[serde(with = "time::serde::rfc3339")]
    pub from: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub thru: OffsetDateTime,
    #[serde(flatten)]
    pub outcome: Outcome<T>,
}
