use crate::Db;
use crate::types::{
    BidderId, DateTime, DemandId, DemandRow, PortfolioId, PortfolioRow, ProductId, ValueRow,
};
use fts_core::models::{DateTimeRangeQuery, DateTimeRangeResponse, PortfolioGroup};
use fts_core::{
    models::{DemandCurve, DemandCurveDto, DemandGroup, ProductGroup},
    ports::{BatchRepository, Solver},
};
use tokio::try_join;

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
        solver: T,
        state: T::State,
    ) -> Result<Result<(), T::Error>, Self::Error> {
        // TH
        let demand_records =
            sqlx::query_file_as!(DemandRow::<()>, "queries/active_demands.sql", timestamp)
                .fetch_all(&self.reader);

        let portfolio_records = sqlx::query_file_as!(
            PortfolioRow::<()>,
            "queries/active_portfolios.sql",
            timestamp
        )
        .fetch_all(&self.reader);

        let (demand_records, portfolio_records) = try_join!(demand_records, portfolio_records)?;

        let demands = demand_records
            .into_iter()
            .filter_map(|row| {
                row.curve_data
                    .map(|data| (row.id, unsafe { DemandCurve::new_unchecked(data.0) }))
            })
            .collect();

        let portfolios = portfolio_records
            .into_iter()
            .filter_map(|row| {
                if let (Some(demand_group), Some(product_group)) =
                    (row.demand_group, row.product_group)
                {
                    Some((row.id, (demand_group.0, product_group.0)))
                } else {
                    None
                }
            })
            .collect();

        // TODO: we may wish to filter the portfolios we include for administrative reasons./
        // what is the best way to do this? Perhaps we say this is (one of) the responsibilities
        // of the state, e.g. contains a HashSet of the "suspended" portfolio ids, and our solver is
        // responsible.... I actually like this a lot.

        // ^ This fits neatly in the filter map approach above.

        let outcome = solver.solve(demands, portfolios, state).await;

        match outcome {
            Ok((portfolio_outcomes, product_outcomes)) => {
                let portfolio_outcomes = sqlx::types::Json(portfolio_outcomes);
                let product_outcomes = sqlx::types::Json(product_outcomes);
                sqlx::query!(
                    r#"
                    update
                        batch
                    set
                        as_of = $1,
                        portfolio_outcomes = jsonb($2),
                        product_outcomes = jsonb($3)
                    "#,
                    timestamp,
                    portfolio_outcomes,
                    product_outcomes
                )
                .execute(&self.writer)
                .await?;
                Ok(Ok(()))
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
    ) -> Result<DateTimeRangeResponse<T::PortfolioOutcome, Self::DateTime>, Self::Error> {
        let limit_p1 = (limit + 1) as i64;
        let mut rows = sqlx::query_as!(
            ValueRow::<T::PortfolioOutcome>,
            r#"
                select
                    valid_from as "valid_from!: crate::types::DateTime",
                    valid_until as "valid_until?: crate::types::DateTime",
                    json(value) as "value!: sqlx::types::Json<T::PortfolioOutcome>"
                from
                    portfolio_outcome
                where
                    portfolio_id = $1
                and
                    ($2 is null or valid_from >= $2)
                and
                    ($3 is null or valid_until is null or valid_until < $3)
                group by
                    valid_from
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
    ) -> Result<DateTimeRangeResponse<T::ProductOutcome, Self::DateTime>, Self::Error> {
        let limit_p1 = (limit + 1) as i64;
        let mut rows = sqlx::query_as!(
            ValueRow::<T::ProductOutcome>,
            r#"
                select
                    valid_from as "valid_from!: crate::types::DateTime",
                    valid_until as "valid_until?: crate::types::DateTime",
                    json(value) as "value!: sqlx::types::Json<T::ProductOutcome>"
                from
                    product_outcome
                where
                    product_id = $1
                and
                    ($2 is null or valid_from >= $2)
                and
                    ($3 is null or valid_until is null or valid_until < $3)
                group by
                    valid_from
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
