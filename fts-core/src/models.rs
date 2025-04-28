mod auction;
mod auth;
mod bound;
mod config;
mod cost;
mod datetime;
mod demand;
mod outcome;
mod product;
mod submission;

pub use auction::{AuctionMetaData, AuctionSolveRequest, RawAuctionInput};
pub use auth::{AuthData, AuthHistoryRecord, AuthId, AuthRecord, Portfolio};
pub use bound::Bound;
pub use config::Config;
pub use cost::{CostData, CostHistoryRecord, CostId, CostRecord, Group, GroupDisplay};
pub use datetime::{DateTimeRangeQuery, DateTimeRangeResponse};
pub use demand::{Constant, Curve, DemandCurve, Point};
pub use outcome::{AuctionOutcome, Outcome};
pub use product::{ProductData, ProductQuery, ProductQueryResponse, ProductRecord};
pub use submission::SubmissionRecord;

macro_rules! uuid_wrapper {
    ($struct: ident) => {
        /// A UUID newtype
        #[derive(
            Debug,
            Hash,
            PartialEq,
            Eq,
            Clone,
            Copy,
            serde::Serialize,
            serde::Deserialize,
            PartialOrd,
            Ord,
            utoipa::ToSchema,
        )]
        #[serde(transparent)]
        #[repr(transparent)]
        pub struct $struct(uuid::Uuid);

        impl From<uuid::Uuid> for $struct {
            fn from(value: uuid::Uuid) -> Self {
                Self(value)
            }
        }

        impl Into<uuid::Uuid> for $struct {
            fn into(self) -> uuid::Uuid {
                self.0
            }
        }

        impl TryFrom<&str> for $struct {
            type Error = <uuid::Uuid as std::str::FromStr>::Err;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Ok(Self(<uuid::Uuid as std::str::FromStr>::from_str(value)?))
            }
        }

        impl Into<String> for $struct {
            fn into(self) -> String {
                self.0.to_string()
            }
        }

        impl std::ops::Deref for $struct {
            type Target = uuid::Uuid;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::fmt::Display for $struct {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

pub(crate) use uuid_wrapper;
uuid_wrapper!(BidderId);
uuid_wrapper!(ProductId);

macro_rules! map_wrapper {
    ($struct:ident, $key:ty, $value:ty) => {
        /// A hashmap with deterministic ordering
        #[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
        #[serde(transparent)]
        #[schema(value_type = std::collections::HashMap<$key, $value>)]
        pub struct $struct(pub indexmap::IndexMap<$key, $value, rustc_hash::FxBuildHasher>);

        impl std::ops::Deref for $struct {
            type Target = indexmap::IndexMap<$key, $value, rustc_hash::FxBuildHasher>;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl IntoIterator for $struct {
            type Item = ($key, $value);
            type IntoIter = indexmap::map::IntoIter<$key, $value>;
            /// Forward the into_iter() implementation from the newtype
            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }

        impl FromIterator<($key, $value)> for $struct {
            fn from_iter<I: IntoIterator<Item = ($key, $value)>>(iter: I) -> Self {
                Self(indexmap::IndexMap::from_iter(iter))
            }
        }
    };
}

pub(crate) use map_wrapper;
