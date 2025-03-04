use rusqlite::ToSql;
use rusqlite::types::FromSql;
use std::borrow::Borrow;
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

/// This type acts as a bridge between `fts-core`'s use of `OffsetDateTime` and
/// how SQLite stores timestamps. Whenever we read or store a timestamp, it
/// should go through this wrapper to ensure consistency.
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

impl ToSql for DateTime {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

impl FromSql for DateTime {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        PrimitiveDateTime::column_result(value).map(|dt| Self(dt))
    }
}
