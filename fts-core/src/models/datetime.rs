use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};

/// A query type for dealing with datetime ranges
///
/// This structure enables API endpoints to accept parameters for filtering results
/// based on time ranges with optional upper and lower bounds.
#[derive(Serialize, Deserialize, Debug, ToSchema, IntoParams)]
pub struct DateTimeRangeQuery {
    /// The upper bound (exclusive) for the datetime range
    #[serde(
        default,
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub before: Option<OffsetDateTime>,

    /// The lower bound (inclusive) for the datetime range
    #[serde(
        default,
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub after: Option<OffsetDateTime>,
}

/// The paginated response to a datetime query
///
/// This structure provides a standard format for returning time-based paginated results,
/// including both the results and pagination metadata for retrieving the next page.
#[derive(Serialize, Deserialize, Debug, ToSchema)]
pub struct DateTimeRangeResponse<T> {
    /// The collection of results matching the query
    pub results: Vec<T>,

    /// Optional pagination metadata for retrieving the next page of results.
    /// If present, indicates there are more results available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub more: Option<DateTimeRangeQuery>,
}
