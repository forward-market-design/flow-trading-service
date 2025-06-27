//! DateTime type for temporal data in flow trading.
//!
//! This module provides a [`DateTime`] type that represents UTC timestamps with
//! subsecond precision. It wraps `time::PrimitiveDateTime` and ensures all
//! serialization happens in RFC3339 format for consistency across the system.

use std::{borrow::Borrow, fmt::Display};
use time::format_description::well_known::Rfc3339;

/// A type that represents a datetime with subsecond precision.
///
/// This type is used throughout the flow trading system to represent timestamps
/// for events, validity periods, and historical records. It ensures:
///
/// - All times are stored and processed in UTC
/// - Serialization/deserialization uses RFC3339 format
/// - SQLite storage uses the appropriate datetime type
///
/// # Examples
///
/// ```
/// # use fts_sqlite::types::DateTime;
/// # use time::OffsetDateTime;
/// let now = OffsetDateTime::now_utc();
/// let datetime = DateTime::from(now);
/// println!("{}", datetime); // Prints in RFC3339 format
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    sqlx::Type,
)]
#[serde(from = "DateTimeDto", into = "DateTimeDto")]
#[sqlx(transparent)]
pub struct DateTime(time::PrimitiveDateTime);

impl Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value: time::OffsetDateTime = self.clone().into();
        write!(f, "{}", value.format(&Rfc3339).unwrap())
    }
}

impl<T: Borrow<time::OffsetDateTime>> From<T> for DateTime {
    fn from(value: T) -> Self {
        let utc = value.borrow().to_offset(time::UtcOffset::UTC);
        Self(time::PrimitiveDateTime::new(utc.date(), utc.time()))
    }
}

impl Into<time::OffsetDateTime> for DateTime {
    fn into(self) -> time::OffsetDateTime {
        self.0.assume_utc()
    }
}

// This is a helper type that ensures (de)serialization happens with respect to RFC3339

#[derive(serde::Serialize, serde::Deserialize)]
struct DateTimeDto(#[serde(with = "time::serde::rfc3339")] time::OffsetDateTime);

impl From<DateTimeDto> for DateTime {
    fn from(value: DateTimeDto) -> Self {
        value.0.into()
    }
}

impl Into<DateTimeDto> for DateTime {
    fn into(self) -> DateTimeDto {
        DateTimeDto(self.into())
    }
}

#[cfg(feature = "schemars")]
impl schemars::JsonSchema for DateTime {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> std::borrow::Cow<'static, str> {
        "DateTime".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "format": "date-time",
        })
    }
}
