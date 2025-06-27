use crate::Db;
use crate::types::{BatchData, DemandId, OutcomeRow, PortfolioId, ProductId};
use fts_core::models::{DateTimeRangeQuery, DateTimeRangeResponse, OutcomeRecord};
use fts_core::{
    models::{DemandCurve, DemandCurveDto, Map},
    ports::{BatchRepository, Solver},
};

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
        // TODO: we may wish to filter the portfolios we include for administrative reasons./
        // what is the best way to do this? Perhaps we say this is (one of) the responsibilities
        // of the state, e.g. contains a HashSet of the "suspended" portfolio ids, and our solver is
        // responsible.... I actually like this a lot.
        let data = sqlx::query_file_as!(BatchData, "queries/gather_batch.sql", timestamp)
            .fetch_optional(&self.reader)
            .await?;

        let (demand_curves, portfolios) = if let Some(BatchData {
            demand_curves,
            demand_groups,
            product_groups,
        }) = data
        {
            let demand_curves = demand_curves
                .map(|x| x.0)
                .unwrap_or_default()
                .into_iter()
                .map(|(key, value)| (key, unsafe { DemandCurve::new_unchecked(value) }))
                .collect();

            let demand_groups = demand_groups.map(|x| x.0).unwrap_or_default();
            let mut product_groups = product_groups.map(|x| x.0).unwrap_or_default();

            // We unify the groups by taking the demand groups and trying to steal a corresponding product group.
            let mut portfolios = demand_groups
                .into_iter()
                .map(|(portfolio_id, demand_group)| {
                    let product_group = product_groups
                        .swap_remove(&portfolio_id)
                        .unwrap_or_default();
                    (portfolio_id, (demand_group, product_group))
                })
                .collect::<Map<_, _>>();
            // Probably there are no product groups left, but if there are we know there was no associated demand group.
            for (portfolio_id, product_group) in product_groups.into_iter() {
                portfolios.insert(portfolio_id, (Default::default(), product_group));
            }

            (demand_curves, portfolios)
        } else {
            Default::default()
        };

        let outcome = solver.solve(demand_curves, portfolios, state).await;

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
    ) -> Result<
        DateTimeRangeResponse<OutcomeRecord<Self::DateTime, T::PortfolioOutcome>, Self::DateTime>,
        Self::Error,
    > {
        let limit_p1 = (limit + 1) as i64;
        let mut rows = sqlx::query_as!(
            OutcomeRow::<T::PortfolioOutcome>,
            r#"
                select
                    valid_from as "as_of!: crate::types::DateTime",
                    json(value) as "outcome!: sqlx::types::Json<T::PortfolioOutcome>"
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
                before: Some(extra.as_of),
                after: query.after,
            })
        } else {
            None
        };

        Ok(DateTimeRangeResponse {
            results: rows
                .into_iter()
                .map(|row| OutcomeRecord {
                    as_of: row.as_of,
                    outcome: row.outcome.0,
                })
                .collect(),
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
    ) -> Result<
        DateTimeRangeResponse<OutcomeRecord<Self::DateTime, T::ProductOutcome>, Self::DateTime>,
        Self::Error,
    > {
        let limit_p1 = (limit + 1) as i64;
        let mut rows = sqlx::query_as!(
            OutcomeRow::<T::ProductOutcome>,
            r#"
                select
                    valid_from as "as_of!: crate::types::DateTime",
                    json(value) as "outcome!: sqlx::types::Json<T::ProductOutcome>"
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
                before: Some(extra.as_of),
                after: query.after,
            })
        } else {
            None
        };

        Ok(DateTimeRangeResponse {
            results: rows
                .into_iter()
                .map(|row| OutcomeRecord {
                    as_of: row.as_of,
                    outcome: row.outcome.0,
                })
                .collect(),
            more,
        })
    }
}
