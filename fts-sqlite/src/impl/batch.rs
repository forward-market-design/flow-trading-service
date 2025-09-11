use crate::Db;
use crate::types::{DateTime, DemandId, PortfolioId, ProductId, ValueRow};
use fts_core::models::{BatchConfig, DateTimeRangeQuery, DateTimeRangeResponse};
use fts_core::ports::Outcome;
use fts_core::{
    models::{Basis, DemandCurve, DemandCurveDto, Weights},
    ports::{BatchRepository, Solver},
};
use tokio::try_join;

struct ActiveDemand {
    id: DemandId,
    expires: Option<DateTime>,
    value: sqlx::types::Json<DemandCurveDto>,
}

impl ActiveDemand {
    fn curve(self) -> DemandCurve {
        // SAFETY: we only serialize validated demand curves
        unsafe { DemandCurve::new_unchecked(self.value.0) }
    }
}

struct ActivePortfolio {
    id: PortfolioId,
    expires: Option<DateTime>,
    demand: sqlx::types::Json<Weights<DemandId>>,
    basis: sqlx::types::Json<Basis<ProductId>>,
}

fn coalesce_min<T: Copy + Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    a.or(b).min(b.or(a))
}

impl<T: Solver<DemandId, PortfolioId, ProductId>> BatchRepository<T> for Db
where
    T: Send,
    T::Error: Send,
    T::State: Send,
    T::PortfolioOutcome: Unpin + Send + serde::Serialize + serde::de::DeserializeOwned,
    T::ProductOutcome: Unpin + Send + serde::Serialize + serde::de::DeserializeOwned,
{
    async fn run_batch(
        &self,
        timestamp: Self::DateTime,
        config: BatchConfig,
        solver: T,
        state: T::State,
    ) -> Result<Result<Option<Self::DateTime>, T::Error>, Self::Error> {
        let demand_records =
            sqlx::query_file_as!(ActiveDemand, "queries/active_demands.sql", timestamp)
                .fetch_all(&self.reader);

        let portfolio_records =
            sqlx::query_file_as!(ActivePortfolio, "queries/active_portfolios.sql", timestamp)
                .fetch_all(&self.reader);

        let (demand_records, portfolio_records) = try_join!(demand_records, portfolio_records)?;

        let mut expires = coalesce_min(
            demand_records.get(0).map(|x| x.expires).flatten(),
            portfolio_records
                .get(0)
                .map(|portfolio| portfolio.expires)
                .flatten(),
        );

        let demands = demand_records
            .into_iter()
            .map(|row| {
                expires = coalesce_min(expires, row.expires);
                (row.id, row.curve())
            })
            .collect();

        let portfolios = portfolio_records
            .into_iter()
            .map(|row| {
                expires = coalesce_min(expires, row.expires);
                (row.id, (row.demand.0, row.basis.0))
            })
            .collect();

        // TODO: we may wish to filter the portfolios we include for administrative reasons./
        // what is the best way to do this? Perhaps we say this is (one of) the responsibilities
        // of the state, e.g. contains a HashSet of the "suspended" portfolio ids, and our solver is
        // responsible.... I actually like this a lot.

        let outcome = solver.solve(demands, portfolios, state).await;

        match outcome {
            Ok((portfolio_outcomes, product_outcomes)) => {
                let portfolio_outcomes = sqlx::types::Json(portfolio_outcomes);
                let product_outcomes = sqlx::types::Json(product_outcomes);
                let time_unit_in_ms = config.time_unit.as_secs_f64() * 1000f64;
                sqlx::query!(
                    r#"
                    insert into
                        batch (valid_from, portfolio_outcomes, product_outcomes, time_unit_in_ms)
                    values
                        ($1, jsonb($2), jsonb($3), $4)
                    "#,
                    timestamp,
                    portfolio_outcomes,
                    product_outcomes,
                    time_unit_in_ms,
                )
                .execute(&self.writer)
                .await?;
                Ok(Ok(expires))
            }
            Err(error) => Ok(Err(error)),
        }
    }

    /// Get the portfolio's outcomes
    ///
    /// This returns a list of outcomes, each corresponding to a specific point in time.
    /// The records are ordered by `valid_from` in descending order
    /// and are grouped by `valid_from`.
    async fn get_portfolio_outcomes(
        &self,
        portfolio_id: Self::PortfolioId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> Result<DateTimeRangeResponse<Outcome<T::PortfolioOutcome>, Self::DateTime>, Self::Error>
    {
        let limit_p1 = (limit + 1) as i64;
        let mut rows = sqlx::query_as!(
            ValueRow::<Outcome<T::PortfolioOutcome>>,
            r#"
                select
                    valid_from as "valid_from!: crate::types::DateTime",
                    valid_until as "valid_until?: crate::types::DateTime",
                    json_object('trade', trade, 'price', price, 'data', data) as "value!: sqlx::types::Json<Outcome<T::PortfolioOutcome>>"
                from
                    batch
                join
                    batch_portfolio
                on
                    batch.id = batch_portfolio.batch_id
                where
                    portfolio_id = $1
                and
                    ($2 is null or valid_from >= $2)
                and
                    ($3 is null or valid_until is null or valid_until < $3)
                order by
                    valid_from desc
                limit $4
            "#,
            portfolio_id,
            query.after,
            query.before,
            limit_p1,
        )
        .fetch_all(&self.reader)
        .await?;

        let more = if rows.len() == limit + 1 {
            let extra = rows.pop().unwrap();
            Some(DateTimeRangeQuery {
                before: Some(extra.valid_from),
                after: query.after,
            })
        } else {
            None
        };

        Ok(DateTimeRangeResponse {
            results: rows.into_iter().map(Into::into).collect(),
            more,
        })
    }

    /// Get the product's outcomes
    ///
    /// This returns a list of outcomes, each corresponding to a specific point in time.
    /// The records are ordered by `valid_from` in descending order
    /// and are grouped by `valid_from`.
    async fn get_product_outcomes(
        &self,
        product_id: Self::ProductId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> Result<DateTimeRangeResponse<Outcome<T::ProductOutcome>, Self::DateTime>, Self::Error>
    {
        let limit_p1 = (limit + 1) as i64;
        let mut rows = sqlx::query_as!(
            ValueRow::<Outcome<T::ProductOutcome>>,
            r#"
                select
                    valid_from as "valid_from!: crate::types::DateTime",
                    valid_until as "valid_until?: crate::types::DateTime",
                    json_object('trade', trade, 'price', price, 'data', data) as "value!: sqlx::types::Json<Outcome<T::ProductOutcome>>"
                from
                    batch
                join
                    batch_product
                on
                    batch.id = batch_product.batch_id
                where
                    product_id = $1
                and
                    ($2 is null or valid_from >= $2)
                and
                    ($3 is null or valid_until is null or valid_until < $3)
                order by
                    valid_from desc
                limit $4
            "#,
            product_id,
            query.after,
            query.before,
            limit_p1,
        )
        .fetch_all(&self.reader)
        .await?;

        let more = if rows.len() == limit + 1 {
            let extra = rows.pop().unwrap();
            Some(DateTimeRangeQuery {
                before: Some(extra.valid_from),
                after: query.after,
            })
        } else {
            None
        };

        Ok(DateTimeRangeResponse {
            results: rows.into_iter().map(Into::into).collect(),
            more,
        })
    }
}
