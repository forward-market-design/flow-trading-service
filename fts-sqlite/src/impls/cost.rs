use std::{borrow::Borrow, str::FromStr};

use crate::{DateTime, db};
use fts_core::{
    models::{
        AuthId, BidderId, CostData, CostHistoryRecord, CostId, CostRecord, DateTimeRangeQuery,
        DateTimeRangeResponse, GroupDisplay,
    },
    ports::{CostFailure, CostRepository},
};
use sqlx::{Executor, Sqlite, Transaction};
use time::OffsetDateTime;

impl CostRepository for db::Database {
    async fn create<K: Borrow<AuthId>, V: Borrow<f64>, P: Borrow<(K, V)>>(
        &self,
        bidder_id: BidderId,
        cost_id: Option<CostId>,
        group: impl Iterator<Item = P>,
        cost: CostData,
        timestamp: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> Result<Result<CostRecord, CostFailure>, Self::Error> {
        let mut tx: Transaction<_> = self.begin().await?;

        let record = {
            // Create the cost and get an option of its ID
            let id_option =
                create_cost(&mut *tx, bidder_id, cost_id, group, cost, timestamp).await?;

            // Try to get the cost record if we have an ID
            let cost_record = if let Some(id) = id_option {
                get_cost(&mut *tx, id, timestamp, include_group).await?
            } else {
                None
            };

            tx.commit().await?;
            cost_record
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
        let mut conn = self.acquire().await?;

        let bidder_id_other: Option<BidderId> =
            sqlx::query_scalar!("SELECT bidder_id FROM cost WHERE id = ?", *cost_id)
                .fetch_optional(&mut *conn)
                .await?
                .map(|bytes| uuid::Uuid::from_slice(&bytes).unwrap().into());

        match bidder_id_other {
            Some(bidder_id_other) if bidder_id_other == bidder_id => {
                let rec = get_cost(&mut *conn, cost_id, as_of, include_group)
                    .await?
                    .ok_or(CostFailure::DoesNotExist);
                Ok(rec)
            }
            Some(_) => Ok(Err(CostFailure::AccessDenied)),
            None => Ok(Err(CostFailure::DoesNotExist)),
        }
    }

    async fn update(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        data: CostData,
        timestamp: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> Result<Result<CostRecord, CostFailure>, Self::Error> {
        let mut tx: Transaction<'_, Sqlite> = self.begin().await?;

        let bidder_id_other: Option<BidderId> =
            sqlx::query_scalar!("SELECT bidder_id FROM cost WHERE id = ?", *cost_id)
                .fetch_optional(&mut *tx)
                .await?
                .map(|bytes| uuid::Uuid::from_slice(&bytes).unwrap().into());

        match bidder_id_other {
            Some(bidder_id_other) if bidder_id_other == bidder_id => {
                let content = serde_json::to_value(data.clone())?;
                let t = DateTime::from(timestamp);
                sqlx::query!(
                    "INSERT INTO cost_data (cost_id, version, content) VALUES (?, ?, ?)",
                    *cost_id,
                    t,
                    content
                )
                .execute(&mut *tx)
                .await?;

                let rec = get_cost(&mut *tx, cost_id, timestamp, include_group)
                    .await?
                    .ok_or(CostFailure::DoesNotExist);

                tx.commit().await?;

                Ok(rec)
            }
            Some(_) => Ok(Err(CostFailure::AccessDenied)),
            None => Ok(Err(CostFailure::DoesNotExist)),
        }
    }

    async fn delete(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        timestamp: OffsetDateTime,
        include_group: GroupDisplay,
    ) -> Result<Result<CostRecord, CostFailure>, Self::Error> {
        let mut tx: Transaction<'_, Sqlite> = self.begin().await?;

        let bidder_id_other: Option<BidderId> =
            sqlx::query_scalar!("SELECT bidder_id FROM cost WHERE id = ?", *cost_id)
                .fetch_optional(&mut *tx)
                .await?
                .map(|bytes| uuid::Uuid::from_slice(&bytes).unwrap().into());

        match bidder_id_other {
            Some(bidder_id_other) if bidder_id_other == bidder_id => {
                let t = DateTime::from(timestamp);
                sqlx::query!(
                    "INSERT INTO cost_data (cost_id, version, content) VALUES (?, ?, NULL)",
                    *cost_id,
                    t
                )
                .execute(&mut *tx)
                .await?;

                let rec = get_cost(&mut *tx, cost_id, timestamp, include_group)
                    .await?
                    .ok_or(CostFailure::DoesNotExist);

                tx.commit().await?;

                Ok(rec)
            }
            Some(_) => Ok(Err(CostFailure::AccessDenied)),
            None => Ok(Err(CostFailure::DoesNotExist)),
        }
    }

    async fn get_history(
        &self,
        bidder_id: BidderId,
        cost_id: CostId,
        query: DateTimeRangeQuery,
        limit: usize,
    ) -> Result<Result<DateTimeRangeResponse<CostHistoryRecord>, CostFailure>, Self::Error> {
        let mut conn = self.acquire().await?;

        let bidder_id_other: Option<BidderId> =
            sqlx::query_scalar!("SELECT bidder_id FROM cost WHERE id = ?", *cost_id)
                .fetch_optional(&mut *conn)
                .await?
                .map(|bytes| uuid::Uuid::from_slice(&bytes).unwrap().into());

        match bidder_id_other {
            Some(bidder_id_other) if bidder_id_other == bidder_id => {
                let before = query.before.map(DateTime::from);
                let after = query.after.map(DateTime::from);
                let lim = (limit + 1) as i64;
                let rows = sqlx::query!(
                    r#"
                    SELECT content, version
                    FROM cost_data
                    WHERE cost_id = ?
                    AND (? IS NULL OR version <= ?)
                    AND (? IS NULL OR version >= ?)
                    ORDER BY version DESC
                    LIMIT ?
                    "#,
                    *cost_id,
                    before,
                    before,
                    after,
                    after,
                    lim
                )
                .fetch_all(&mut *conn)
                .await?;

                let mut results = rows
                    .into_iter()
                    .map(|row| {
                        Ok(CostHistoryRecord {
                            data: match &row.content {
                                Some(content_str) => serde_json::from_str(content_str)?,
                                None => serde_json::from_value(serde_json::Value::Null)?,
                            },
                            version: DateTime::from_str(&row.version).unwrap().into(),
                        })
                    })
                    .collect::<Result<Vec<_>, db::Error>>()?;

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
            Some(_) => Ok(Err(CostFailure::AccessDenied)),
            None => Ok(Err(CostFailure::DoesNotExist)),
        }
    }
}

pub async fn create_cost<K: Borrow<AuthId>, V: Borrow<f64>, P: Borrow<(K, V)>>(
    mut executor: impl Executor<'_, Database = Sqlite>,
    bidder_id: BidderId,
    cost_id: Option<CostId>,
    group: impl Iterator<Item = P>,
    cost: CostData,
    timestamp: OffsetDateTime,
) -> Result<Option<CostId>, db::Error> {
    // Generate a new random id if not provided
    let id = cost_id.unwrap_or_else(|| uuid::Uuid::new_v4().into());

    // Do an existence check for the id
    let exists = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM cost WHERE id = ?)", *id)
        .fetch_one(&mut *executor)
        .await?;

    if exists != 0 {
        return Ok(None);
    }

    // Now we can go about writing the cost row...
    sqlx::query!(
        "INSERT INTO cost (id, bidder_id) VALUES (?, ?)",
        *id,
        *bidder_id
    )
    .execute(&mut *executor)
    .await?;

    // ... and the cost data.
    sqlx::query!(
        "INSERT INTO cost_data (cost_id, version, content) VALUES (?, ?, ?)",
        *id,
        DateTime::from(timestamp),
        serde_json::to_value(cost)?
    )
    .execute(&mut *executor)
    .await?;

    // Now write the group weights
    for pair in group {
        let (auth_id, weight) = pair.borrow();
        sqlx::query!(
            "INSERT INTO cost_weight (cost_id, auth_id, weight) \
             VALUES (?, ?, ?) ON CONFLICT (cost_id, auth_id) DO UPDATE \
             SET weight = cost_weight.weight + excluded.weight",
            *id,
            **auth_id.borrow(),
            *weight.borrow()
        )
        .execute(&mut *executor)
        .await?;
    }

    Ok(Some(id))
}

pub async fn get_cost(
    mut executor: impl Executor<'_, Database = Sqlite>,
    cost_id: CostId,
    as_of: OffsetDateTime,
    group: GroupDisplay,
) -> Result<Option<CostRecord>, db::Error> {
    let row = sqlx::query!(
        r#"
        SELECT content, version, bidder_id
        FROM cost_data
        JOIN cost
        ON cost_data.cost_id = cost.id
        WHERE cost_id = ?
        AND version <= ?
        ORDER BY version DESC
        LIMIT 1
        "#,
        *cost_id,
        DateTime::from(as_of)
    )
    .fetch_optional(&mut *executor)
    .await?;

    if let Some(row) = row {
        let group = match group {
            GroupDisplay::Exclude => None,
            GroupDisplay::Include => {
                let rows = sqlx::query!(
                    r#"
                    SELECT auth_id, weight
                    FROM cost_weight
                    WHERE cost_id = ?
                    AND weight != 0.0
                    ORDER BY auth_id
                    "#,
                    *cost_id
                )
                .fetch_all(&mut *executor)
                .await?;

                let entries = rows
                    .into_iter()
                    .map(|row| Ok((AuthId::from(row.auth_id), row.weight)))
                    .collect::<Result<Vec<_>, db::Error>>()?;

                Some(entries)
            }
        };

        Ok(Some(CostRecord {
            bidder_id: BidderId::from(row.bidder_id),
            cost_id,
            group,
            data: serde_json::from_value(row.content)?,
            version: DateTime::from(row.version).into(),
        }))
    } else {
        Ok(None)
    }
}
