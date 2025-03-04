use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};

#[derive(Serialize, Deserialize, Debug, ToSchema, IntoParams)]
pub struct DateTimeRangeQuery {
    #[serde(
        default,
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub before: Option<OffsetDateTime>,
    #[serde(
        default,
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub after: Option<OffsetDateTime>,
}

#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct DateTimeRangeResponse<T> {
    pub results: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub more: Option<DateTimeRangeQuery>,
}
