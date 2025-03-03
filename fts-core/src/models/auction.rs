use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use time::{Duration, OffsetDateTime};
use utoipa::ToSchema;

use super::{AuthId, AuthRecord, BidderId, CostRecord, ProductId};

/// Configuration for scheduling and running a batch auction.
///
/// Flow trading uses batch auctions to clear markets at regular intervals.
/// This structure defines the time parameters for scheduling these auctions.
#[derive(Deserialize, ToSchema)]
pub struct AuctionSolveRequest {
    /// The starting time of the batch; if omitted, defaults to the last batch's ending time
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub from: Option<OffsetDateTime>,

    /// The ending time of the batch
    #[serde(with = "time::serde::rfc3339")]
    pub thru: OffsetDateTime,

    /// Optionally divide the (from, thru) interval into smaller batches with the provided duration
    ///
    /// This allows for scheduling multiple consecutive sub-auctions within the given time range.
    #[serde(default, deserialize_with = "optional_humantime")]
    pub by: Option<Duration>,
}

/// A simple struct for the external identification of an auction.
///
/// Provides the time interval of an auction for external reference.
#[derive(Serialize, ToSchema)]
pub struct AuctionMetaData {
    /// The starting time of the auction interval
    #[serde(with = "time::serde::rfc3339")]
    pub from: OffsetDateTime,
    /// The ending time of the auction interval
    #[serde(with = "time::serde::rfc3339")]
    pub thru: OffsetDateTime,
}

/// A struct containing the raw auth and cost records for the stated auction interval.
///
/// This structure collects all the inputs needed to solve an auction for a specific time interval.
/// It contains all auths and costs that are active during the interval, as well as timing information
/// needed to scale rate-based quantities appropriately.
pub struct RawAuctionInput<AuctionId> {
    /// An internal auction id
    pub id: AuctionId,
    /// The start time for this batch
    pub from: OffsetDateTime,
    /// The end time for this batch
    pub thru: OffsetDateTime,
    /// All appropriate auth records for the interval
    pub auths: Vec<AuthRecord>,
    /// All appropriate cost records for the interval
    pub costs: Vec<CostRecord>,
    /// The reference time that all rate-based quantities are defined with respect to
    pub trade_duration: Duration,
}

impl<T> Into<Vec<fts_solver::Submission<AuthId, ProductId>>> for RawAuctionInput<T> {
    fn into(self) -> Vec<fts_solver::Submission<AuthId, ProductId>> {
        // Convert the auction into a format the solver understands
        // First, we aggregate the auths by the bidder
        let mut auths_by_bidder = IndexMap::<BidderId, Vec<AuthRecord>, FxBuildHasher>::default();

        for record in self.auths {
            auths_by_bidder
                .entry(record.bidder_id)
                .or_default()
                .push(record);
        }

        // Same story for costs
        let mut costs_by_bidder = IndexMap::<BidderId, Vec<CostRecord>, FxBuildHasher>::default();

        for record in self.costs {
            costs_by_bidder
                .entry(record.bidder_id)
                .or_default()
                .push(record);
        }

        let scale = (self.thru - self.from) / self.trade_duration;

        // Now we produce a list of submissions

        costs_by_bidder
            .into_iter()
            .filter_map(|(bidder_id, costs)| {
                // If the bidder has no auths, then the costs are no-op.
                if let Some(auths) = auths_by_bidder.swap_remove(&bidder_id) {
                    // Otherwise, we scale the rate-based definitions into quantity-based ones
                    let auths = auths
                        .into_iter()
                        .filter_map(|record| record.into_solver(scale));

                    let costs = costs
                        .into_iter()
                        .filter_map(|record| record.into_solver(scale));

                    // Having done all that, we now have a submission
                    fts_solver::Submission::new(auths, costs)
                } else {
                    None
                }
            })
            .collect()
    }
}

// TODO: this is probably unnecessary with the right sequence of invocations,
// but it is easy enough to maintain.

/// Deserializes a human-readable duration string into an Option<Duration>.
///
/// This helper function allows duration values to be specified in a human-friendly format
/// (e.g., "1h", "30m", "1d") in the API requests.
fn optional_humantime<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Duration>, D::Error> {
    // Extract the string, if present
    let value: Option<&str> = Deserialize::deserialize(deserializer)?;

    if let Some(value) = value {
        // If the string is present, try parsing.
        // Note that humantime parses into std::time::Duration, so we also need to do a conversion
        let delta = humantime::parse_duration(value).map_err(serde::de::Error::custom)?;

        let sec: i64 = delta
            .as_secs()
            .try_into()
            .map_err(serde::de::Error::custom)?;
        let ns: i32 = delta
            .subsec_nanos()
            .try_into()
            .map_err(serde::de::Error::custom)?;

        Ok(Some(Duration::new(sec, ns)))
    } else {
        Ok(None)
    }
}
