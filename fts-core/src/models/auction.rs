use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use time::{Duration, OffsetDateTime};
use utoipa::ToSchema;

use super::{AuthId, AuthRecord, BidderId, CostRecord, ProductId};

#[derive(Deserialize, ToSchema)]
pub struct AuctionSolveRequest {
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub from: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub thru: OffsetDateTime,
    #[serde(default, deserialize_with = "optional_humantime")]
    pub by: Option<Duration>,
}

#[derive(Serialize, ToSchema)]
pub struct AuctionMetaData {
    #[serde(with = "time::serde::rfc3339")]
    pub from: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub thru: OffsetDateTime,
}

pub struct RawAuctionInput<AuctionId> {
    pub id: AuctionId,
    pub from: OffsetDateTime,
    pub thru: OffsetDateTime,
    pub auths: Vec<AuthRecord>,
    pub costs: Vec<CostRecord>,
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
