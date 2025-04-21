use std::{borrow::Borrow, ops::Deref as _};

use crate::{DateTime, db};
use fts_core::{
    models::{
        AuthId, BidderId, DemandCurve, CostHistoryRecord, CostId, CostRecord, DateTimeRangeQuery,
        DateTimeRangeResponse, GroupDisplay,
    },
    ports::{CostFailure, CostRepository},
};
use rusqlite::{Connection, OptionalExtension as _, TransactionBehavior};
use time::OffsetDateTime;
use uuid::Uuid;

impl CostRepository for db::Database {
    async fn create<K: Borrow<AuthId>, V: Borrow<f64>, P: Borrow<(K, V)>>(
        &self,
        bidder_id: BidderId,
        cost_id: Option<CostId>,
        group: impl Iterator<Item = P>,
        cost: DemandCurve,
        timestamp: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> Result<Result<CostRecord, CostFailure>, Self::Error> {
        let mut ctx = self.connect(true)?;

        let record = {
            let tx = ctx.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let result = create_cost(&tx, bidder_id, cost_id, group, cost, timestamp)?
                .map(|id| {
                    get_cost(&tx, id, timestamp, include_group)
                        .transpose()
                        .unwrap()
                })
                .transpose()?;
            tx.commit()?;
            result
        };

        match record {
            // todo: inline this into query
            Some(cost) => Ok(Ok(cost)),
            // The only reason this implementation fails to create an cost,
            // given no other db errors, is due to an id conflict
            None => Ok(Err(CostFailure::IdConflict)),
        }
    }

    async fn read(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        as_of: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> Result<Result<CostRecord, CostFailure>, Self::Error> {
        let ctx = self.connect(false)?;

        if let Some(bidder_id_other) = get_bidder(&ctx, cost_id)? {
            if bidder_id_other == bidder_id {
                Ok(Ok(get_cost(&ctx, cost_id, as_of, include_group)?.unwrap()))
            } else {
                Ok(Err(CostFailure::AccessDenied))
            }
        } else {
            Ok(Err(CostFailure::DoesNotExist))
        }
    }

    async fn update(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        data: DemandCurve,
        timestamp: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> Result<Result<CostRecord, CostFailure>, Self::Error> {
        let ctx = self.connect(true)?;

        if let Some(bidder_id_other) = get_bidder(&ctx, cost_id)? {
            if bidder_id_other == bidder_id {
                update_cost(&ctx, cost_id, Some(data), timestamp)?;
                Ok(Ok(
                    get_cost(&ctx, cost_id, timestamp, include_group)?.unwrap()
                ))
            } else {
                Ok(Err(CostFailure::AccessDenied))
            }
        } else {
            Ok(Err(CostFailure::DoesNotExist))
        }
    }

    async fn delete(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        timestamp: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> Result<Result<CostRecord, CostFailure>, Self::Error> {
        let ctx = self.connect(true)?;

        if let Some(bidder_id_other) = get_bidder(&ctx, cost_id)? {
            if bidder_id_other == bidder_id {
                ctx.execute(
                    r#"insert into cost_data (cost_id, version, content) values (?, ?, null)"#,
                    (*cost_id, DateTime::from(timestamp)),
                )?;
                Ok(Ok(
                    get_cost(&ctx, cost_id, timestamp, include_group)?.unwrap()
                ))
            } else {
                Ok(Err(CostFailure::AccessDenied))
            }
        } else {
            Ok(Err(CostFailure::DoesNotExist))
        }
    }

    async fn get_history(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        query: DateTimeRangeQuery,
        limit: usize,
    ) -> Result<Result<DateTimeRangeResponse<CostHistoryRecord>, CostFailure>, Self::Error> {
        let ctx = self.connect(false)?;

        match get_bidder(&ctx, cost_id)? {
            Some(other) => {
                if other != bidder_id {
                    return Ok(Err(CostFailure::AccessDenied));
                }
            }
            None => {
                return Ok(Err(CostFailure::DoesNotExist));
            }
        };

        // Get the paginated history of bids associated to this cost
        let mut stmt = ctx.prepare(
            r#"
            select
                content, version
            from
                cost_data
            join
                cost
            on
                cost_data.cost_id = cost.id
            where
                cost_id = ?1
            and
                (?2 is null or version <= ?2)
            and
                (?3 is null or version >= ?3)
            order by
                version desc
            limit ?4
            "#,
        )?;

        let mut results = stmt
            .query_and_then(
                (
                    *cost_id,
                    query.before.map(DateTime::from),
                    query.after.map(DateTime::from),
                    limit + 1,
                ),
                |row| -> Result<CostHistoryRecord, Self::Error> {
                    Ok(CostHistoryRecord {
                        data: serde_json::from_value(row.get(0)?)?,
                        version: row.get::<usize, DateTime>(1)?.into(),
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        // We paginate by adding 1 to the limit, popping the result of, and
        // using it to adjust the query object
        let more = if results.len() == limit + 1 {
            let extra = results.pop().unwrap();
            Some(DateTimeRangeQuery {
                before: Some(extra.version),
                after: query.after,
            })
        } else {
            None
        };

        Ok(Ok(DateTimeRangeResponse { results, more }))
    }
}

pub fn create_cost<K: Borrow<AuthId>, V: Borrow<f64>, P: Borrow<(K, V)>>(
    ctx: &Connection,
    bidder_id: BidderId,
    cost_id: Option<CostId>,
    group: impl Iterator<Item = P>,
    cost: DemandCurve,
    timestamp: OffsetDateTime,
) -> Result<Option<CostId>, db::Error> {
    // Generate a new random id if not provided
    let id = cost_id.unwrap_or_else(|| uuid::Uuid::new_v4().into());

    // Do an existence check for the id
    let exists: bool = ctx.query_row(
        r#"select exists(select 1 from cost where id = ?)"#,
        (*id,),
        |row| row.get(0),
    )?;
    if exists {
        return Ok(None);
    }

    // Now we can go about writing the cost row...
    ctx.execute(
        r#"insert into cost (id, bidder_id) values (?, ?)"#,
        (id.deref(), bidder_id.deref()),
    )?;

    // ... and the cost data.
    ctx.execute(
        r#"insert into cost_data (cost_id, version, content) values (?, ?, ?)"#,
        (*id, DateTime::from(timestamp), serde_json::to_value(cost)?),
    )?;

    {
        // Now write the group weights
        let mut stmt = ctx.prepare(
            r#"
            insert into
                cost_weight (cost_id, auth_id, weight)
            values
                (?, ?, ?)
            on conflict
                (cost_id, auth_id)
            do
                update set weight = cost_weight.weight + excluded.weight
            "#,
        )?;
        for pair in group {
            let (key, value) = pair.borrow();
            stmt.execute((id.deref(), key.borrow().deref(), value.borrow()))?;
        }
    };

    Ok(Some(id))
}

pub fn get_cost(
    ctx: &Connection,
    cost_id: CostId,
    as_of: OffsetDateTime,
    group: GroupDisplay,
) -> Result<Option<CostRecord>, db::Error> {
    // We defer the bidder check until slightly futher down the implementation

    // Find the matching cost
    let mut stmt = ctx.prepare(
        r#"
            select
                content, version, bidder_id
            from
                cost_data
            join
                cost
            on
                cost_data.cost_id = cost.id
            where
                cost_id = ?1
            and
                version <= ?2
            order by
                version desc
            limit
                1
            "#,
    )?;

    let result = stmt
        .query_and_then(
            (cost_id.deref(), DateTime::from(as_of)),
            |row| -> Result<(Option<DemandCurve>, OffsetDateTime, BidderId), db::Error> {
                Ok((
                    serde_json::from_value(row.get(0)?)?,
                    row.get::<usize, DateTime>(1)?.into(),
                    row.get::<usize, Uuid>(2)?.into(),
                ))
            },
        )?
        .next()
        .transpose()?;

    if let Some((data, version, bidder_id)) = result {
        let group = match group {
            GroupDisplay::Exclude => None,
            GroupDisplay::Include => {
                // Now we just collect the entries
                let mut stmt = ctx.prepare(r#"select auth_id, weight from cost_weight where cost_id = ? and weight != 0.0 order by auth_id"#)?;

                let result = stmt
                    .query_and_then(
                        (cost_id.deref(),),
                        |row| -> Result<(AuthId, f64), db::Error> {
                            Ok((row.get::<usize, Uuid>(0)?.into(), row.get(1)?))
                        },
                    )?
                    .collect::<Result<_, _>>()?;

                Some(result)
            }
        };

        Ok(Some(CostRecord {
            bidder_id,
            cost_id,
            group,
            data,
            version,
        }))
    } else {
        Ok(None)
    }
}

pub fn update_cost(
    ctx: &Connection,
    cost_id: CostId,
    data: Option<DemandCurve>,
    timestamp: OffsetDateTime,
) -> Result<(), db::Error> {
    ctx.execute(
        r#"insert into cost_data (cost_id, version, content) values (?, ?, ?)"#,
        (
            *cost_id,
            DateTime::from(timestamp),
            data.map(serde_json::to_value).transpose()?,
        ),
    )?;
    Ok(())
}

pub fn active_costs(
    ctx: &Connection,
    bidder_id: Option<BidderId>,
    as_of: OffsetDateTime,
    group: GroupDisplay,
) -> Result<Vec<CostRecord>, db::Error> {
    let mut stmt = ctx.prepare(
        r#"
            select
                bidder_id,
                cost_id,
                content,
                version
            from
                cost_data
            join
                cost_data_lifetime
            using
                (id)
            join
                cost
            on
                cost_data.cost_id = cost.id
            where
                content is not null
            and
                (?1 is null or bidder_id = ?1)
            and
                (birth <= ?2) and (death is null or death > ?2)
            order by
                bidder_id, version, cost_id
            "#,
    )?;

    let results = stmt.query_and_then(
        (bidder_id.map(Into::<Uuid>::into), DateTime::from(as_of)),
        |row| -> Result<_, db::Error> {
            let bidder_id = row.get::<usize, Uuid>(0)?.into();
            let cost_id: CostId = row.get::<usize, Uuid>(1)?.into();
            let data = serde_json::from_value(row.get(2)?)?;
            let version = row.get::<usize, DateTime>(3)?.into();
            let group = match group {
                GroupDisplay::Exclude => None,
                GroupDisplay::Include => {
                    let mut group_expansion = ctx.prepare(
                        r#"
                        select
                            auth_id,
                            weight
                        from
                            cost_weight
                        where
                            cost_id = ?
                        order by
                            auth_id
                        "#,
                    )?;

                    let entries = group_expansion
                        .query_and_then(
                            (cost_id.deref(),),
                            |pair| -> Result<(AuthId, f64), db::Error> {
                                Ok((pair.get::<usize, Uuid>(0)?.into(), pair.get(1)?))
                            },
                        )?
                        .collect::<Result<_, _>>()?;

                    Some(entries)
                }
            };

            Ok(CostRecord {
                bidder_id,
                cost_id,
                group,
                data,
                version,
            })
        },
    )?;

    Ok(results.collect::<Result<Vec<_>, db::Error>>()?)
}

fn get_bidder(ctx: &Connection, cost_id: CostId) -> Result<Option<BidderId>, db::Error> {
    Ok(ctx
        .query_row(
            r#"select bidder_id from cost where id = ?"#,
            (*cost_id,),
            |row| row.get::<usize, Uuid>(0),
        )
        .optional()?
        .map(Into::into))
}
