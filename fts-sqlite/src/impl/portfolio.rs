use crate::{
    Db,
    types::{BidderId, DateTime, DemandId, PortfolioId, PortfolioRow, ProductId, ValueRow},
};
use fts_core::{
    models::{
        DateTimeRangeQuery, DateTimeRangeResponse, DemandGroup, PortfolioRecord, ProductGroup,
    },
    ports::PortfolioRepository,
};

impl<PortfolioData: Send + Unpin + serde::Serialize + serde::de::DeserializeOwned>
    PortfolioRepository<PortfolioData> for Db
{
    async fn get_portfolio_bidder_id(
        &self,
        portfolio_id: Self::PortfolioId,
    ) -> Result<Option<Self::BidderId>, Self::Error> {
        sqlx::query_scalar!(
            r#"
            select
                bidder_id as "id!: BidderId"
            from
                portfolio
            where
                id = $1
            "#,
            portfolio_id
        )
        .fetch_optional(&self.reader)
        .await
    }

    async fn query_portfolio(
        &self,
        bidder_ids: &[Self::BidderId],
        as_of: Self::DateTime,
    ) -> Result<Vec<PortfolioRecord<Self, PortfolioData>>, Self::Error> {
        if bidder_ids.len() == 0 {
            Ok(Vec::new())
        } else {
            let bidder_ids = sqlx::types::Json(bidder_ids);
            let query = sqlx::query_file_as!(
                PortfolioRow,
                "queries/active_portfolios.sql",
                bidder_ids,
                as_of,
            )
            .fetch_all(&self.reader)
            .await?;

            Ok(query.into_iter().map(Into::into).collect())
        }
    }

    async fn create_portfolio(
        &self,
        portfolio_id: Self::PortfolioId,
        bidder_id: Self::BidderId,
        app_data: PortfolioData,
        demand_group: DemandGroup<Self::DemandId>,
        product_group: ProductGroup<Self::ProductId>,
        as_of: Self::DateTime,
    ) -> Result<(), Self::Error> {
        let app_data = sqlx::types::Json(app_data);
        let demand_group = sqlx::types::Json(demand_group);
        let product_group = sqlx::types::Json(product_group);
        sqlx::query!(
            r#"
            insert into
                portfolio (id, as_of, bidder_id, app_data, demand_group, product_group)
            values
                ($1, $2, $3, jsonb($4), jsonb($5), jsonb($6))
            "#,
            portfolio_id,
            as_of,
            bidder_id,
            app_data,
            demand_group,
            product_group
        )
        .execute(&self.writer)
        .await?;
        Ok(())
    }

    async fn update_portfolio(
        &self,
        portfolio_id: Self::PortfolioId,
        demand_group: Option<DemandGroup<Self::DemandId>>,
        product_group: Option<ProductGroup<Self::ProductId>>,
        as_of: Self::DateTime,
    ) -> Result<bool, Self::Error> {
        let updated = match (demand_group, product_group) {
            (Some(demand_group), Some(product_group)) => {
                let demand_group = sqlx::types::Json(demand_group);
                let product_group = sqlx::types::Json(product_group);
                let query = sqlx::query!(
                    r#"
                    update
                        portfolio
                    set
                        as_of = $2,
                        demand_group = jsonb($3),
                        product_group = jsonb($4)
                    where
                        id = $1
                    "#,
                    portfolio_id,
                    as_of,
                    demand_group,
                    product_group,
                )
                .execute(&self.writer)
                .await?;
                query.rows_affected() > 0
            }
            (Some(demand_group), None) => {
                let demand_group = sqlx::types::Json(demand_group);
                let query = sqlx::query!(
                    r#"
                    update
                        portfolio
                    set
                        as_of = $2,
                        demand_group = jsonb($3)
                    where
                        id = $1
                    "#,
                    portfolio_id,
                    as_of,
                    demand_group,
                )
                .execute(&self.writer)
                .await?;
                query.rows_affected() > 0
            }
            (None, Some(product_group)) => {
                let product_group = sqlx::types::Json(product_group);
                let query = sqlx::query!(
                    r#"
                    update
                        portfolio
                    set
                        as_of = $2,
                        product_group = jsonb($3)
                    where
                        id = $1
                    "#,
                    portfolio_id,
                    as_of,
                    product_group,
                )
                .execute(&self.writer)
                .await?;
                query.rows_affected() > 0
            }
            (None, None) => false,
        };

        Ok(updated)
    }

    async fn get_portfolio(
        &self,
        portfolio_id: Self::PortfolioId,
        as_of: Self::DateTime,
    ) -> Result<Option<PortfolioRecord<Self, PortfolioData>>, Self::Error> {
        let query = sqlx::query_file_as!(
            PortfolioRow,
            "queries/get_portfolio_by_id.sql",
            portfolio_id,
            as_of
        )
        .fetch_optional(&self.reader)
        .await?;

        Ok(query.map(Into::into))
    }

    /// Get the history of this portfolio's demands
    ///
    /// This returns a list of records, each containing the state of the portfolio's demand group
    /// at a specific point in time. The records are ordered by `valid_from` in descending order
    /// and are grouped by `valid_from`. This is important for a `more` pointer to work correctly,
    /// so the demand_group is actually a map of `demand_id` to `weight` at that point in time.
    async fn get_portfolio_demand_history(
        &self,
        portfolio_id: Self::PortfolioId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> Result<DateTimeRangeResponse<DemandGroup<Self::DemandId>, Self::DateTime>, Self::Error>
    {
        let limit_p1 = (limit + 1) as i64;
        let mut rows = sqlx::query_as!(
            ValueRow::<DemandGroup<DemandId>>,
            r#"
                select
                    valid_from as "valid_from!: crate::types::DateTime",
                    valid_until as "valid_until?: crate::types::DateTime",
                    json_group_object(demand_id, weight) as "value!: sqlx::types::Json<DemandGroup<DemandId>>"
                from
                    demand_group
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

    /// Get the history of this portfolio's products
    ///
    /// This returns a list of records, each containing the state of the portfolio's product group
    /// at a specific point in time. The records are ordered by `valid_from` in descending order
    /// and are grouped by `valid_from`. This is important for a `more` pointer to work correctly,
    /// so the product_group is actually a map of `product_id` to `weight` at that point in time.
    async fn get_portfolio_product_history(
        &self,
        portfolio_id: Self::PortfolioId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> Result<DateTimeRangeResponse<ProductGroup<Self::ProductId>, Self::DateTime>, Self::Error>
    {
        let limit_p1 = (limit + 1) as i64;
        let mut rows = sqlx::query_as!(
            ValueRow::<ProductGroup<ProductId>>,
            r#"
                select
                    valid_from as "valid_from!: crate::types::DateTime",
                    valid_until as "valid_until?: crate::types::DateTime",
                    json_group_object(product_id, weight) as "value!: sqlx::types::Json<ProductGroup<ProductId>>"
                from
                    product_group
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
}
