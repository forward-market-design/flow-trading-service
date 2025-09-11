//! DateTime type for temporal data in flow trading.
//!
//! This module provides a [`DateTime`] type that represents UTC timestamps with
//! subsecond precision. It wraps `time::PrimitiveDateTime` and ensures all
//! serialization happens in RFC3339 format for consistency across the system.

use sqlx::{Database, Decode, Encode, Type};
use std::{fmt::Display, str::FromStr};
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
    serde_with::SerializeDisplay,
    serde_with::DeserializeFromStr,
)]
pub struct DateTime(time::UtcDateTime);

impl Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value: time::OffsetDateTime = self.0.into();
        write!(f, "{}", value.format(&Rfc3339).unwrap())
    }
}

impl FromStr for DateTime {
    type Err = time::error::Parse;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        time::OffsetDateTime::parse(s, &Rfc3339).map(Into::into)
    }
}

impl From<time::OffsetDateTime> for DateTime {
    fn from(value: time::OffsetDateTime) -> Self {
        Self(value.into())
    }
}

impl Into<time::OffsetDateTime> for DateTime {
    fn into(self) -> time::OffsetDateTime {
        self.0.into()
    }
}

impl<'q, DB: Database> Encode<'q, DB> for DateTime
where
    String: Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut <DB as Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        self.to_string().encode_by_ref(buf)
    }
    fn encode(
        self,
        buf: &mut <DB as Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError>
    where
        Self: Sized,
    {
        self.to_string().encode(buf)
    }
}

impl<'r, DB: Database> Decode<'r, DB> for DateTime
where
    String: Decode<'r, DB>,
{
    fn decode(value: <DB as Database>::ValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        String::decode(value).and_then(|s| Ok(Self::from_str(&s)?))
    }
}

impl<DB: Database> Type<DB> for DateTime
where
    String: Type<DB>,
{
    fn type_info() -> <DB as Database>::TypeInfo {
        String::type_info()
    }
    fn compatible(ty: &<DB as Database>::TypeInfo) -> bool {
        String::compatible(ty)
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
