//! Strongly-typed identifier types for flow trading entities.
//!
//! This module provides newtype wrappers around UUIDs for different entity types
//! in the system. Using distinct types for each kind of ID prevents mixing up
//! identifiers at compile time and improves code clarity.
//!
//! All ID types implement:
//! - Serialization/deserialization as transparent UUIDs
//! - SQLite storage as strings
//! - Display formatting
//! - Conversion to/from standard UUIDs

macro_rules! new_id {
    ($struct:ident) => {
        new_id!($struct, "A newtype wrapper around a uuid");
    };
    ($struct:ident, $doc:literal) => {
        #[doc = $doc]
        #[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(transparent)]
        pub struct $struct(pub uuid::Uuid);

        impl Into<uuid::Uuid> for $struct {
            fn into(self) -> uuid::Uuid {
                self.0
            }
        }

        impl From<uuid::Uuid> for $struct {
            fn from(value: uuid::Uuid) -> Self {
                Self(value)
            }
        }

        impl std::fmt::Display for $struct {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $struct {
            type Err = <uuid::Uuid as std::str::FromStr>::Err;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(s.parse()?))
            }
        }

        impl sqlx::Type<sqlx::Sqlite> for $struct {
            fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
                <String as sqlx::Type<sqlx::Sqlite>>::type_info()
            }
        }

        impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for $struct {
            fn encode_by_ref(
                &self,
                args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                sqlx::Encode::<'q, sqlx::Sqlite>::encode_by_ref(&self.0.to_string(), args)
            }
        }

        impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for $struct {
            fn decode(
                value: sqlx::sqlite::SqliteValueRef<'r>,
            ) -> Result<Self, sqlx::error::BoxDynError> {
                let string = <&str as sqlx::Decode<'r, sqlx::Sqlite>>::decode(value)?;
                let value = string.parse()?;
                Ok(value)
            }
        }
    };
}

new_id!(
    BidderId,
    "Unique identifier for a bidder in the flow trading system"
);
new_id!(DemandId, "Unique identifier for a demand curve submission");
new_id!(
    PortfolioId,
    "Unique identifier for a portfolio that groups demands and products"
);
new_id!(ProductId, "Unique identifier for a tradeable product");
