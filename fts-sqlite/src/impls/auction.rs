use super::{
    auth::{PortfolioOptions, active_auths},
    cost::active_costs,
};
use crate::{DateTime, db};
use fts_core::{
    models::{AuctionMetaData, AuthId, Config, GroupDisplay, Outcome, ProductId, RawAuctionInput},
    ports::AuctionRepository,
};
use rusqlite::{OptionalExtension, TransactionBehavior};
use std::ops::Deref;
use time::{Duration, OffsetDateTime};

impl AuctionRepository for db::Database {
    type AuctionId = i64;

    fn config(&self) -> &Config {
        self.config()
    }

    fn solver() -> impl fts_solver::Solver + Send {
        fts_solver::clarabel::ClarabelSolver::default()
    }

    async fn prepare(
        &self,
        from: Option<OffsetDateTime>,
        thru: OffsetDateTime,
        by: Option<Duration>,
        timestamp: OffsetDateTime,
    ) -> Result<Option<Vec<RawAuctionInput<Self::AuctionId>>>, Self::Error> {
        let ctx = self.connect(false)?;

        // If we are given from, no need to find the last_auction time.
        // Otherwise, query the db. If we're still out of luck, use thru - by.
        // Otherwise, just use the timestamp.
        let from = if let Some(from) = from {
            from.clone()
        } else if let Some(from) = ctx
            .query_row(
                r#"select "thru" from auction where "thru" < ? order by "thru" desc limit 1"#,
                (DateTime::from(thru),),
                |row| row.get::<usize, DateTime>(0).map(Into::into),
            )
            .optional()?
        {
            from
        } else if let Some(by) = by {
            thru - by
        } else {
            timestamp.clone()
        };

        // Sanity check
        if from >= thru {
            return Ok(None);
        }

        // How long is each batch?
        let auction_duration = by.map(|x| x.clone()).unwrap_or(thru - from);

        // What units are the bids specified in?
        let reference_duration = self.config().trade_rate.try_into().unwrap();

        // Collect the batches to run
        let mut submissions = Vec::new();
        let mut as_of = from;
        while as_of + auction_duration <= thru {
            let from = DateTime::from(as_of);
            let thru = DateTime::from(as_of + auction_duration);
            let auths = active_auths(&ctx, None, as_of, PortfolioOptions::Expand)?;
            let costs = active_costs(&ctx, None, as_of, GroupDisplay::Include)?;
            submissions.push((from, thru, auths, costs));
            as_of += auction_duration;
        }

        std::mem::drop(ctx);
        let ts = DateTime::from(timestamp);
        let ctx = self.connect(true)?;

        Ok(Some(submissions.into_iter().map(|(from, thru, auths, costs)| {
            let auction_id = ctx.query_row(r#"insert into auction ("from", "thru", queued, auction) values (?, ?, ?, ?) returning id"#, (&from, &thru, &ts, serde_json::to_value((&auths, &costs))?), |row| row.get(0))?;
            Ok(RawAuctionInput {
                id: auction_id,
                from: from.into(),
                thru: thru.into(),
                auths,
                costs,
                trade_duration: reference_duration
            })
        }).collect::<Result<Vec<_>, Self::Error>>()?))
    }

    async fn report(
        &self,
        auction_id: Self::AuctionId,
        auth_outcomes: impl Iterator<Item = (AuthId, Outcome<()>)>,
        product_outcomes: impl Iterator<Item = (ProductId, Outcome<()>)>,
        timestamp: OffsetDateTime,
    ) -> Result<Option<AuctionMetaData>, Self::Error> {
        let mut ctx = self.connect(true)?;
        let tx = ctx.transaction_with_behavior(TransactionBehavior::Immediate)?;

        let metadata = tx.query_row(
            r#"update auction set solved = ?2 where id = ?1 and solved is null returning "from", "thru""#,
            (auction_id, DateTime::from(timestamp)),
            |row| {
                let from: DateTime = row.get(0)?;
                let thru: DateTime = row.get(1)?;
                Ok(AuctionMetaData {
                    from: from.into(),
                    thru: thru.into(),
                })
            }
        ).optional()?;

        if metadata.is_some() {
            // Insert the portfolio results
            {
                let mut stmt = tx.prepare(
                    r#"
                    insert into
                        auth_outcome (auction_id, auth_id, price, trade)
                    values
                        (?, ?, ?, ?)
                    "#,
                )?;
                for (auth_id, outcome) in auth_outcomes {
                    stmt.execute((auction_id, auth_id.deref(), outcome.price, outcome.trade))?;
                }
            };

            // Insert the product summaries
            {
                let mut stmt = tx.prepare(
                    r#"insert into product_outcome (auction_id, product_id, price, trade) values (?, ?, ?, ?)"#,
                )?;
                for (product_id, outcome) in product_outcomes {
                    stmt.execute((auction_id, product_id.deref(), outcome.price, outcome.trade))?;
                }
            };
        }

        tx.commit()?;
        Ok(metadata)
    }
}
