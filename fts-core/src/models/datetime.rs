/// A timestamped record of some value.
///
/// The interval for which the entity has this value is provided alongside
/// the value itself.
#[cfg_attr(
    feature = "schemars",
    derive(schemars::JsonSchema),
    schemars(rename = "{Value}Record")
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ValueRecord<DateTime, Value> {
    /// The timestamp when this change occurred
    pub valid_from: DateTime,
    /// The timestamp when this change was superceded
    pub valid_until: Option<DateTime>,
    /// The component value
    pub value: Value,
}

/// A query type for dealing with datetime ranges
///
/// This structure enables API endpoints to accept parameters for filtering results
/// based on time ranges with optional upper and lower bounds.
#[cfg_attr(
    feature = "schemars",
    derive(schemars::JsonSchema),
    schemars(rename = "DateTimeRangeQuery")
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DateTimeRangeQuery<DateTime> {
    /// The upper bound (exclusive) for the datetime range
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub before: Option<DateTime>,

    /// The lower bound (inclusive) for the datetime range
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub after: Option<DateTime>,
}

/// The paginated response to a datetime query
///
/// This structure provides a standard format for returning time-based paginated results,
/// including both the results and pagination metadata for retrieving the next page.
#[cfg_attr(
    feature = "schemars",
    derive(schemars::JsonSchema),
    schemars(rename = "DateTimeRangeResponse_of_{T}")
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DateTimeRangeResponse<T, DateTime> {
    /// The collection of results matching the query
    pub results: Vec<ValueRecord<DateTime, T>>,

    /// Optional pagination metadata for retrieving the next page of results.
    /// If present, indicates there are more results available.
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub more: Option<DateTimeRangeQuery<DateTime>>,
}
