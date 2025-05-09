use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::sqlite::SqliteArgumentValue;
use sqlx::{Decode, Encode, Sqlite, Type, sqlite::SqliteValueRef};
use std::borrow::Borrow;
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

/// This type acts as a bridge between `fts-core`'s use of `OffsetDateTime` and
/// how SQLite stores timestamps. Whenever we read or store a timestamp, it
/// should go through this wrapper to ensure consistency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateTime(PrimitiveDateTime);

impl<T: Borrow<OffsetDateTime>> From<T> for DateTime {
    fn from(value: T) -> Self {
        let utc = value.borrow().to_offset(UtcOffset::UTC);
        Self(PrimitiveDateTime::new(utc.date(), utc.time()))
    }
}

impl Into<OffsetDateTime> for DateTime {
    fn into(self) -> OffsetDateTime {
        self.0.assume_utc()
    }
}

// Tell SQLx that DateTime should be treated as a TEXT type in SQLite
impl Type<Sqlite> for DateTime {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

// Implement encoding for SQLx (to save to database)
impl Encode<'_, Sqlite> for DateTime {
    fn encode_by_ref(&self, buf: &mut Vec<SqliteArgumentValue<'_>>) -> Result<IsNull, BoxDynError> {
        // Convert PrimitiveDateTime to RFC3339 string
        let dt_string = self
            .0
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| Box::new(e) as BoxDynError)?;

        // Encode the string
        <String as Encode<Sqlite>>::encode(dt_string, buf)
    }
}

// Implement decoding for SQLx (to load from database)
impl<'r> Decode<'r, Sqlite> for DateTime {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        // Decode as string
        let text = <String as Decode<Sqlite>>::decode(value)?;

        // Parse RFC3339 string to PrimitiveDateTime
        let dt = PrimitiveDateTime::parse(&text, &time::format_description::well_known::Rfc3339)?;

        Ok(DateTime(dt))
    }
}
