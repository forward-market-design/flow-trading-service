mod auction;
mod auth;
mod bound;
mod cost;
mod datetime;
mod outcome;
mod product;
mod submission;

pub use auction::{AuctionMetaData, AuctionSolveRequest, RawAuctionInput};
pub use auth::{AuthData, AuthHistoryRecord, AuthId, AuthRecord, Portfolio};
pub use bound::Bound;
pub use cost::{
    Constant, CostData, CostDto, CostDtoCreate, CostDtoRead, CostDtoUpdate, CostHistoryRecord,
    CostId, CostRecord, Curve, Group, GroupDisplay, Point,
};
pub use datetime::{DateTimeRangeQuery, DateTimeRangeResponse};
pub use outcome::{AuctionOutcome, Outcome};
pub use product::{ProductQueryResponse, ProductRecord};
pub use submission::SubmissionRecord;

#[macro_export]
macro_rules! uuid_wrapper {
    ($struct: ident) => {
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

uuid_wrapper!(BidderId);
uuid_wrapper!(ProductId);
