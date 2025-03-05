use super::{
    auth::{PortfolioOptions, active_auths, create_auth, get_auth, update_auth},
    cost::{active_costs, create_cost, get_cost, update_cost},
};
use crate::db;
use fts_core::{
    models::{AuthId, AuthRecord, BidderId, CostId, CostRecord, GroupDisplay, SubmissionRecord},
    ports::{
        AuthFailure, CostFailure, SubmissionAuthDto, SubmissionCostDto, SubmissionDto,
        SubmissionFailure, SubmissionRepository,
    },
};
use rusqlite::TransactionBehavior;
use time::OffsetDateTime;

type Map<K, V> = indexmap::IndexMap<K, V, fxhash::FxBuildHasher>;

impl SubmissionRepository for db::Database {
    async fn get_submission(
        &self,
        bidder_id: BidderId,
        as_of: OffsetDateTime,
    ) -> Result<SubmissionRecord, Self::Error> {
        let ctx = self.connect(false)?;

        // We just reuse the underlying implementations for the AuthRepository and CostRepository,
        // but use the same database connection.
        let auths = active_auths(&ctx, Some(bidder_id), as_of, Default::default())?;
        let costs = active_costs(&ctx, Some(bidder_id), as_of, Default::default())?;

        Ok(SubmissionRecord {
            auths,
            costs,
            as_of,
        })
    }

    async fn set_submission(
        &self,
        bidder_id: BidderId,
        submission: SubmissionDto,
        timestamp: OffsetDateTime,
    ) -> Result<Result<SubmissionRecord, SubmissionFailure>, Self::Error> {
        let mut ctx = self.connect(true)?;
        let tx = ctx.transaction_with_behavior(TransactionBehavior::Immediate)?;

        let mut current_auths: Map<AuthId, AuthRecord> =
            active_auths(&tx, Some(bidder_id), timestamp, PortfolioOptions::Exclude)?
                .into_iter()
                .map(|record| (record.auth_id, record))
                .collect();

        let mut current_costs: Map<CostId, CostRecord> =
            active_costs(&tx, Some(bidder_id), timestamp, GroupDisplay::Exclude)?
                .into_iter()
                .map(|record| (record.cost_id, record))
                .collect();

        let mut new_auths = Map::<AuthId, AuthRecord>::with_capacity_and_hasher(
            submission.auths.len(),
            Default::default(),
        );

        let mut new_costs: Vec<CostRecord> = Vec::with_capacity(submission.costs.len());

        // Parse the auths

        for auth_dto in submission.auths {
            let auth_id = auth_dto.auth_id();

            let current_auth = current_auths
                .swap_remove(&auth_id)
                .map(|auth| Ok(auth))
                .or_else(|| {
                    get_auth(&tx, auth_id, timestamp, PortfolioOptions::Exclude).transpose()
                })
                .transpose()?;

            // Before going any further, make sure the bidders line up
            let bidder_conflict = current_auth
                .as_ref()
                .map(|auth| auth.bidder_id != bidder_id)
                .unwrap_or(false);
            if bidder_conflict {
                return Ok(Err(SubmissionFailure::Auth(AuthFailure::AccessDenied)));
            }

            // Now we get the new auth; creating, updating, or reading as appropriate
            let new_auth = match auth_dto {
                SubmissionAuthDto::Read { .. } => {
                    if let Some(auth) = current_auth {
                        auth
                    } else {
                        return Ok(Err(SubmissionFailure::Auth(AuthFailure::DoesNotExist)));
                    }
                }
                SubmissionAuthDto::Create {
                    portfolio, data, ..
                } => {
                    create_auth(
                        &tx,
                        bidder_id,
                        Some(auth_id),
                        portfolio.into_iter(),
                        data,
                        timestamp,
                    )?;
                    get_auth(&tx, auth_id, timestamp, PortfolioOptions::Exclude)?.unwrap()
                }
                SubmissionAuthDto::Update { data, .. } => {
                    update_auth(&tx, auth_id, Some(data), timestamp)?;
                    get_auth(&tx, auth_id, timestamp, PortfolioOptions::Exclude)?.unwrap()
                }
            };

            new_auths.insert(auth_id, new_auth);
        }

        for cost_dto in submission.costs {
            let cost_id = cost_dto.cost_id();

            let current_cost = current_costs
                .swap_remove(&cost_id)
                .map(|cost| Ok(cost))
                .or_else(|| get_cost(&tx, cost_id, timestamp, GroupDisplay::Exclude).transpose())
                .transpose()?;

            // Before going any further, make sure the bidders line up
            let bidder_conflict = current_cost
                .as_ref()
                .map(|cost| cost.bidder_id != bidder_id)
                .unwrap_or(false);
            if bidder_conflict {
                return Ok(Err(SubmissionFailure::Cost(CostFailure::AccessDenied)));
            }

            let new_cost = match cost_dto {
                SubmissionCostDto::Read { .. } => {
                    if let Some(cost) = current_cost {
                        cost
                    } else {
                        return Ok(Err(SubmissionFailure::Cost(CostFailure::DoesNotExist)));
                    }
                }
                SubmissionCostDto::Create { group, data, .. } => {
                    create_cost(
                        &tx,
                        bidder_id,
                        Some(cost_id),
                        group.into_iter(),
                        data,
                        timestamp,
                    )?;
                    get_cost(&tx, cost_id, timestamp, GroupDisplay::Exclude)?.unwrap()
                }
                SubmissionCostDto::Update { data, .. } => {
                    update_cost(&tx, cost_id, Some(data), timestamp)?;
                    get_cost(&tx, cost_id, timestamp, GroupDisplay::Exclude)?.unwrap()
                }
            };

            new_costs.push(new_cost);
        }

        for auth in current_auths.into_values() {
            update_auth(&tx, auth.auth_id, None, timestamp)?;
        }
        for cost in current_costs.into_values() {
            update_cost(&tx, cost.cost_id, None, timestamp)?;
        }

        tx.commit()?;

        Ok(Ok(SubmissionRecord {
            auths: new_auths.into_values().collect(),
            costs: new_costs,
            as_of: timestamp,
        }))
    }
}
