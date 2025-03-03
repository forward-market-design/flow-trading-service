use crate::{DateTime, db};
use fts_core::{
    models::{
        AuctionOutcome, AuthData, AuthHistoryRecord, AuthId, AuthRecord, BidderId,
        DateTimeRangeQuery, DateTimeRangeResponse, Outcome, Portfolio, PortfolioDisplay, ProductId,
    },
    ports::{AuthFailure, AuthRepository},
};
use rusqlite::{Connection, OptionalExtension as _, Statement, TransactionBehavior};
use std::{borrow::Borrow, iter, marker::PhantomData, ops::Deref};
use time::OffsetDateTime;
use uuid::Uuid;

impl AuthRepository for db::Database {
    async fn create<K: Borrow<ProductId>, V: Borrow<f64>, P: Borrow<(K, V)>>(
        &self,
        bidder_id: BidderId,
        auth_id: Option<AuthId>,
        portfolio: impl Iterator<Item = P>,
        data: AuthData,
        timestamp: OffsetDateTime,
        include_portfolio: PortfolioDisplay,
    ) -> Result<Result<AuthRecord, AuthFailure>, Self::Error> {
        let mut ctx = self.connect(true)?;

        let record = {
            let tx = ctx.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let result = create_auth(&tx, bidder_id, auth_id, portfolio, data, timestamp)?
                .map(|id| {
                    get_auth(&tx, id, timestamp, include_portfolio)
                        .transpose()
                        .unwrap()
                })
                .transpose()?;
            tx.commit()?;
            result
        };

        match record {
            Some(auth) => Ok(Ok(auth)),
            // The only reason this implementation fails to create an auth,
            // given no other db errors, is due to an id conflict
            None => Ok(Err(AuthFailure::IdConflict)),
        }
    }

    async fn read(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        as_of: OffsetDateTime,
        include_portfolio: PortfolioDisplay,
    ) -> Result<Result<AuthRecord, AuthFailure>, Self::Error> {
        let ctx = self.connect(false)?;

        if let Some(bidder_id_other) = get_bidder(&ctx, auth_id)? {
            if bidder_id_other == bidder_id {
                Ok(Ok(
                    get_auth(&ctx, auth_id, as_of, include_portfolio)?.unwrap()
                ))
            } else {
                Ok(Err(AuthFailure::AccessDenied))
            }
        } else {
            Ok(Err(AuthFailure::DoesNotExist))
        }
    }

    async fn update(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        data: AuthData,
        timestamp: OffsetDateTime,
        include_portfolio: PortfolioDisplay,
    ) -> Result<Result<AuthRecord, AuthFailure>, Self::Error> {
        let ctx = self.connect(true)?;

        if let Some(bidder_id_other) = get_bidder(&ctx, auth_id)? {
            if bidder_id_other == bidder_id {
                update_auth(&ctx, auth_id, Some(data), timestamp)?;
                Ok(Ok(
                    get_auth(&ctx, auth_id, timestamp, include_portfolio)?.unwrap()
                ))
            } else {
                Ok(Err(AuthFailure::AccessDenied))
            }
        } else {
            Ok(Err(AuthFailure::DoesNotExist))
        }
    }

    async fn delete(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        timestamp: OffsetDateTime,
        include_portfolio: PortfolioDisplay,
    ) -> Result<Result<AuthRecord, AuthFailure>, Self::Error> {
        let ctx = self.connect(true)?;

        if let Some(bidder_id_other) = get_bidder(&ctx, auth_id)? {
            if bidder_id_other == bidder_id {
                ctx.execute(
                    r#"insert into auth_data (auth_id, version, content) values (?, ?, null)"#,
                    (*auth_id, DateTime::from(timestamp)),
                )?;
                Ok(Ok(
                    get_auth(&ctx, auth_id, timestamp, include_portfolio)?.unwrap()
                ))
            } else {
                Ok(Err(AuthFailure::AccessDenied))
            }
        } else {
            Ok(Err(AuthFailure::DoesNotExist))
        }
    }

    async fn get_history(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        query: DateTimeRangeQuery,
        limit: usize,
    ) -> Result<Result<DateTimeRangeResponse<AuthHistoryRecord>, AuthFailure>, Self::Error> {
        let ctx = self.connect(false)?;

        match get_bidder(&ctx, auth_id)? {
            Some(other) => {
                if other != bidder_id {
                    return Ok(Err(AuthFailure::AccessDenied));
                }
            }
            None => {
                return Ok(Err(AuthFailure::DoesNotExist));
            }
        };

        // Get the paginated history of data associated to this cost
        let mut stmt = ctx.prepare(
            r#"
            select
                content, version
            from
                auth_data
            join
                auth
            on
                auth_data.auth_id = auth.id
            where
                auth_id = ?1
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
                    *auth_id,
                    query.before.map(DateTime::from),
                    query.after.map(DateTime::from),
                    limit + 1,
                ),
                |row| -> Result<AuthHistoryRecord, db::Error> {
                    Ok(AuthHistoryRecord {
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

    async fn get_outcomes(
        &self,
        bidder_id: BidderId,
        auth_id: AuthId,
        query: DateTimeRangeQuery,
        limit: usize,
    ) -> Result<Result<DateTimeRangeResponse<AuctionOutcome<()>>, AuthFailure>, Self::Error> {
        let ctx = self.connect(false)?;

        match get_bidder(&ctx, auth_id)? {
            Some(other) => {
                if other != bidder_id {
                    return Ok(Err(AuthFailure::AccessDenied));
                }
            }
            None => {
                return Ok(Err(AuthFailure::DoesNotExist));
            }
        };

        let mut stmt = ctx.prepare(
            r#"
            select
                "from",
                "thru",
                price,
                trade
            from
                auth_outcome
            join
                auction
            on
                auth_outcome.auction_id = auction.id
            join
                auth
            on
                auth_outcome.auth_id = auth.id
            where
                auth_id = ?1
            and
                (?2 is null or "from" <= ?2)
            and
                (?3 is null or ?3 >= "from")
            order by
                "from" desc
            limit ?4
            "#,
        )?;

        let mut results = stmt
            .query_and_then(
                (
                    auth_id.deref(),
                    query.before.map(DateTime::from),
                    query.after.map(DateTime::from),
                    limit + 1,
                ),
                |row| -> Result<AuctionOutcome<()>, db::Error> {
                    Ok(AuctionOutcome {
                        from: row.get::<usize, DateTime>(0)?.into(),
                        thru: row.get::<usize, DateTime>(1)?.into(),
                        outcome: Outcome {
                            price: row.get(2)?,
                            trade: row.get(3)?,
                            data: None,
                        },
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        // We paginate by adding 1 to the limit, popping the result of, and
        // using it to adjust the query object
        let more = if results.len() == limit + 1 {
            let extra = results.pop().unwrap();
            Some(DateTimeRangeQuery {
                before: Some(extra.from),
                after: query.after,
            })
        } else {
            None
        };

        Ok(Ok(DateTimeRangeResponse { results, more }))
    }

    async fn query_by_product(
        &self,
        bidder_id: BidderId,
        product_id: ProductId,
        as_of: OffsetDateTime,
    ) -> Result<Vec<AuthRecord>, Self::Error> {
        let ctx = self.connect(false)?;

        // Find the matching auth_data
        let mut stmt = ctx.prepare(
            r#"
            select
                id, content, max(version) as version, weight
            from (
                select
                    auth.id as id,
                    auth_weight.weight as weight
                from
                    auth
                join
                    auth_weight
                on
                    auth.id = auth_weight.auth_id
                where
                    auth.bidder_id = ?1
                and
                    auth_weight.product_id = ?2
                and
                    auth_weight.weight != 0.0
            ) as ids
            join
                auth_data
            on
                auth_data.auth_id = ids.id
            where
                auth_data.version <= ?3
            group by
                ids.id
            having
                content is not null
            order by
                id
            "#,
        )?;

        let mut trade_stmt = TradeCalculation::prepare(&ctx)?;

        let results = stmt
            .query_and_then(
                (bidder_id.deref(), product_id.deref(), DateTime::from(as_of)),
                |row| -> Result<AuthRecord, Self::Error> {
                    let bidder_id = bidder_id.clone();
                    let auth_id = row.get::<usize, Uuid>(0)?.into();
                    let portfolio = Some(iter::once((product_id.clone(), row.get(3)?)).collect());
                    let data = Some(serde_json::from_value(row.get(1)?)?);
                    let version = row.get::<usize, DateTime>(2)?.into();
                    let trade = Some(trade_stmt.execute(auth_id, as_of)?);
                    Ok(AuthRecord {
                        bidder_id,
                        auth_id,
                        portfolio,
                        data,
                        version,
                        trade,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }
}

pub fn create_auth<K: Borrow<ProductId>, V: Borrow<f64>, P: Borrow<(K, V)>>(
    ctx: &Connection,
    bidder_id: BidderId,
    auth_id: Option<AuthId>,
    portfolio: impl Iterator<Item = P>,
    data: AuthData,
    timestamp: OffsetDateTime,
) -> Result<Option<AuthId>, db::Error> {
    // Generate a new random id if not provided
    let id = auth_id.unwrap_or_else(|| uuid::Uuid::new_v4().into());

    // Do an existence check for the id
    let exists: bool = ctx.query_row(
        r#"select exists(select 1 from auth where id = ?)"#,
        (*id,),
        |row| row.get(0),
    )?;
    if exists {
        return Ok(None);
    }

    // Now we can go about writing the auth row...
    ctx.execute(
        r#"insert into auth (id, bidder_id) values (?, ?)"#,
        (id.deref(), bidder_id.deref()),
    )?;

    // ... and the auth data.
    ctx.execute(
        r#"insert into auth_data (auth_id, version, content) values (?, ?, ?)"#,
        (*id, DateTime::from(timestamp), serde_json::to_value(data)?),
    )?;

    {
        // Now write the auth weights
        let mut stmt = ctx.prepare(
            r#"
                insert into
                    auth_weight (auth_id, product_id, weight)
                values
                    (?, ?, ?)
                on conflict
                    (auth_id, product_id)
                do
                    update set weight = auth_weight.weight + excluded.weight
                "#,
        )?;
        for pair in portfolio {
            let (key, value) = pair.borrow();
            stmt.execute((id.deref(), key.borrow().deref(), value.borrow()))?;
        }
    }

    Ok(Some(id))
}

// does no permissions checks
pub fn get_auth(
    ctx: &Connection,
    auth_id: AuthId,
    as_of: OffsetDateTime,
    portfolio: PortfolioDisplay,
) -> Result<Option<AuthRecord>, db::Error> {
    // We defer the bidder check until slightly futher down the implementation

    // Find the matching auth_data
    let mut stmt = ctx.prepare(
        r#"
                select
                    content, version, bidder_id
                from
                    auth_data
                join
                    auth
                on
                    auth_data.auth_id = auth.id
                where
                    auth_id = ?1
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
            (auth_id.deref(), DateTime::from(as_of)),
            |row| -> Result<(Option<AuthData>, OffsetDateTime, BidderId), db::Error> {
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
        // Provide the definition if requested
        let portfolio = match portfolio {
            PortfolioDisplay::Exclude => None,
            PortfolioDisplay::Include => Some(PortfolioDefinition::simple(ctx)?.execute(auth_id)?),
            PortfolioDisplay::Expand => Some(PortfolioDefinition::full(ctx)?.execute(auth_id)?),
        };

        let trade = Some(TradeCalculation::prepare(ctx)?.execute(auth_id, as_of)?);

        Ok(Some(AuthRecord {
            bidder_id,
            auth_id,
            portfolio,
            data,
            version,
            trade,
        }))
    } else {
        Ok(None)
    }
}

pub fn update_auth(
    ctx: &Connection,
    auth_id: AuthId,
    data: Option<AuthData>,
    timestamp: OffsetDateTime,
) -> Result<(), db::Error> {
    ctx.execute(
        r#"insert into auth_data (auth_id, version, content) values (?, ?, ?)"#,
        (
            *auth_id,
            DateTime::from(timestamp),
            data.map(serde_json::to_value).transpose()?,
        ),
    )?;
    Ok(())
}

struct PortfolioDefinition<'t>(Statement<'t>, PhantomData<ProductId>);

impl<'t> PortfolioDefinition<'t> {
    fn simple(ctx: &'t Connection) -> Result<Self, db::Error> {
        Ok(Self(
            ctx.prepare(
                r#"
                select
                    product_id as id,
                    weight
                from
                    auth_weight
                where
                    auth_id = ?
                order by
                    id
                "#,
            )?,
            PhantomData,
        ))
    }

    fn full(ctx: &'t Connection) -> Result<Self, db::Error> {
        Ok(Self(
            ctx.prepare(
                r#"
                select
                    id,
                    sum(weight) as weight
                from (
                    select
                        A.dst as id,
                        auth_weight.weight * A.ratio as weight
                    from
                        auth_weight
                    join
                        product_tree as A
                    on
                        auth_weight.product_id = A.src
                    join
                        product_tree as B
                    on
                        A.dst = B.src
                    where
                        auth_weight.auth_id = ?
                    group by
                        A.src, B.src
                    having
                        count(*) = 1
                )
                group by
                    id
                order by
                    id
                "#,
            )?,
            PhantomData,
        ))
    }

    fn execute(&mut self, auth_id: AuthId) -> Result<Portfolio, db::Error> {
        let result = self
            .0
            .query_and_then(
                (auth_id.deref(),),
                |row| -> Result<(ProductId, f64), db::Error> {
                    Ok((row.get::<usize, Uuid>(0)?.into(), row.get(1)?))
                },
            )?
            .collect::<Result<Portfolio, _>>()?;

        Ok(result)
    }
}

struct TradeCalculation<'t>(Statement<'t>, PhantomData<(AuthId, OffsetDateTime)>);

impl<'t> TradeCalculation<'t> {
    fn prepare(ctx: &'t Connection) -> Result<Self, db::Error> {
        Ok(Self(
            ctx.prepare(
                r#"
                select
                    total(trade)
                from
                    auth_outcome
                join
                    auction
                on
                    auth_outcome.auction_id = auction.id
                where
                    auth_outcome.auth_id = ?1
                and
                    auction."from" <= ?2
                "#,
            )?,
            PhantomData,
        ))
    }

    fn execute(&mut self, auth_id: AuthId, as_of: OffsetDateTime) -> Result<f64, db::Error> {
        let result = self
            .0
            .query_row((auth_id.deref(), DateTime::from(as_of)), |row| row.get(0))?;
        Ok(result)
    }
}

pub fn active_auths(
    ctx: &Connection,
    bidder_id: Option<BidderId>,
    as_of: OffsetDateTime,
    portfolio: PortfolioDisplay,
) -> Result<Vec<AuthRecord>, db::Error> {
    let mut populator = match portfolio {
        PortfolioDisplay::Exclude => None,
        PortfolioDisplay::Include => Some(PortfolioDefinition::simple(ctx)?),
        PortfolioDisplay::Expand => Some(PortfolioDefinition::full(ctx)?),
    };

    let mut trade_calculator = TradeCalculation::prepare(ctx)?;

    let mut stmt = ctx.prepare(
        r#"
        select
            bidder_id,
            auth_id,
            content,
            version
        from
            auth_data
        join
            auth_data_lifetime
        using
            (id)
        join
            auth
        on
            auth_data.auth_id = auth.id
        where
            content is not null
        and
            (?1 is null or bidder_id = ?1)
        and
            (birth <= ?2) and (death is null or death > ?2)
        order by
            bidder_id, version, auth_id
        "#,
    )?;

    let results = stmt.query_and_then(
        (bidder_id.map(Into::<Uuid>::into), DateTime::from(as_of)),
        |row| -> Result<_, db::Error> {
            let auth_id = row.get::<usize, Uuid>(1)?.into();
            let portfolio = populator.as_mut().map(|p| p.execute(auth_id)).transpose()?;

            Ok(AuthRecord {
                bidder_id: row.get::<usize, Uuid>(0)?.into(),
                auth_id,
                portfolio,
                data: serde_json::from_value(row.get(2)?)?,
                version: row.get::<usize, DateTime>(3)?.into(),
                trade: Some(trade_calculator.execute(auth_id, as_of)?),
            })
        },
    )?;

    Ok(results.collect::<Result<Vec<_>, db::Error>>()?)
}

fn get_bidder(ctx: &Connection, auth_id: AuthId) -> Result<Option<BidderId>, db::Error> {
    Ok(ctx
        .query_row(
            r#"select bidder_id from auth where id = ?"#,
            (*auth_id,),
            |row| row.get::<usize, Uuid>(0),
        )
        .optional()?
        .map(Into::into))
}
