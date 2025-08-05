use crate::{
    Db,
    types::{BidderId, DateTime, DemandId, PortfolioId, PortfolioRow, ProductId, ValueRow},
};
use fts_core::{
    models::{Basis, DateTimeRangeQuery, DateTimeRangeResponse, DemandGroup, PortfolioRecord},
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
    ) -> Result<Vec<PortfolioRecord<Self, PortfolioData>>, Self::Error> {
        if bidder_ids.len() == 0 {
            Ok(Vec::new())
        } else {
            let bidder_ids = sqlx::types::Json(bidder_ids);
            let query = sqlx::query_as!(
                PortfolioRow,
                r#"
                select
                    portfolio.id as "id!: PortfolioId",
                    as_of as "valid_from!: DateTime",
                    null as "valid_until?: DateTime",
                    bidder_id as "bidder_id!: BidderId",
                    json(app_data) as "app_data!: sqlx::types::Json<PortfolioData>",
                    json(demand_group) as "demand_group?: sqlx::types::Json<DemandGroup<DemandId>>",
                    json(basis) as "basis?: sqlx::types::Json<Basis<ProductId>>"
                from
                    portfolio
                join
                    json_each($1) as bidder_ids
                on
                    portfolio.bidder_id = bidder_ids.atom
                where
                    portfolio.demand_group is not null
                or
                    portfolio.basis is not null
                "#,
                bidder_ids
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
        basis: Basis<Self::ProductId>,
        as_of: Self::DateTime,
    ) -> Result<PortfolioRecord<Self, PortfolioData>, Self::Error> {
        let app_data = sqlx::types::Json(app_data);
        let demand_group = if demand_group.is_empty() {
            None
        } else {
            Some(sqlx::types::Json(demand_group))
        };
        let basis = if basis.is_empty() {
            None
        } else {
            Some(sqlx::types::Json(basis))
        };
        let portfolio = sqlx::query_as!(
            PortfolioRow,
            r#"
            insert into
                portfolio (id, as_of, bidder_id, app_data, demand_group, basis)
            values
                ($1, $2, $3, jsonb($4), jsonb($5), jsonb($6))
            returning
                id as "id!: PortfolioId",
                as_of as "valid_from!: DateTime",
                null as "valid_until?: DateTime",
                bidder_id as "bidder_id!: BidderId",
                json(app_data) as "app_data!: sqlx::types::Json<PortfolioData>",
                json(demand_group) as "demand_group?: sqlx::types::Json<DemandGroup<DemandId>>",
                json(basis) as "basis?: sqlx::types::Json<Basis<ProductId>>"
            "#,
            portfolio_id,
            as_of,
            bidder_id,
            app_data,
            demand_group,
            basis
        )
        .fetch_one(&self.writer)
        .await?;
        Ok(portfolio.into())
    }

    async fn update_portfolio_demand_group(
        &self,
        portfolio_id: Self::PortfolioId,
        demand_group: DemandGroup<Self::DemandId>,
        as_of: Self::DateTime,
    ) -> Result<Option<PortfolioRecord<Self, PortfolioData>>, Self::Error> {
        let demand_group = if demand_group.is_empty() {
            None
        } else {
            Some(sqlx::types::Json(demand_group))
        };
        let updated = sqlx::query_as!(
            PortfolioRow,
            r#"
            update
                portfolio
            set
                as_of = $2,
                demand_group = jsonb($3)
            where
                id = $1
            returning
                id as "id!: PortfolioId",
                as_of as "valid_from!: DateTime",
                null as "valid_until?: DateTime",
                bidder_id as "bidder_id!: BidderId",
                json(app_data) as "app_data!: sqlx::types::Json<PortfolioData>",
                json(demand_group) as "demand_group?: sqlx::types::Json<DemandGroup<DemandId>>",
                json(basis) as "basis?: sqlx::types::Json<Basis<ProductId>>"
            "#,
            portfolio_id,
            as_of,
            demand_group,
        )
        .fetch_optional(&self.writer)
        .await?;

        Ok(updated.map(Into::into))
    }

    async fn update_portfolio_basis(
        &self,
        portfolio_id: Self::PortfolioId,
        basis: Basis<Self::ProductId>,
        as_of: Self::DateTime,
    ) -> Result<Option<PortfolioRecord<Self, PortfolioData>>, Self::Error> {
        let basis = if basis.is_empty() {
            None
        } else {
            Some(sqlx::types::Json(basis))
        };
        let updated = sqlx::query_as!(
            PortfolioRow,
            r#"
            update
                portfolio
            set
                as_of = $2,
                basis = jsonb($3)
            where
                id = $1
            returning
                id as "id!: PortfolioId",
                as_of as "valid_from!: DateTime",
                null as "valid_until?: DateTime",
                bidder_id as "bidder_id!: BidderId",
                json(app_data) as "app_data!: sqlx::types::Json<PortfolioData>",
                json(demand_group) as "demand_group?: sqlx::types::Json<DemandGroup<DemandId>>",
                json(basis) as "basis?: sqlx::types::Json<Basis<ProductId>>"
            "#,
            portfolio_id,
            as_of,
            basis,
        )
        .fetch_optional(&self.writer)
        .await?;

        Ok(updated.map(Into::into))
    }

    async fn update_portfolio_groups(
        &self,
        portfolio_id: Self::PortfolioId,
        demand_group: DemandGroup<Self::DemandId>,
        basis: Basis<Self::ProductId>,
        as_of: Self::DateTime,
    ) -> Result<Option<PortfolioRecord<Self, PortfolioData>>, Self::Error> {
        let demand_group = if demand_group.is_empty() {
            None
        } else {
            Some(sqlx::types::Json(demand_group))
        };
        let basis = if basis.is_empty() {
            None
        } else {
            Some(sqlx::types::Json(basis))
        };
        let updated = sqlx::query_as!(
            PortfolioRow,
            r#"
            update
                portfolio
            set
                as_of = $2,
                demand_group = jsonb($3),
                basis = jsonb($4)
            where
                id = $1
            returning
                id as "id!: PortfolioId",
                as_of as "valid_from!: DateTime",
                null as "valid_until?: DateTime",
                bidder_id as "bidder_id!: BidderId",
                json(app_data) as "app_data!: sqlx::types::Json<PortfolioData>",
                json(demand_group) as "demand_group?: sqlx::types::Json<DemandGroup<DemandId>>",
                json(basis) as "basis?: sqlx::types::Json<Basis<ProductId>>"
            "#,
            portfolio_id,
            as_of,
            demand_group,
            basis,
        )
        .fetch_optional(&self.writer)
        .await?;

        Ok(updated.map(Into::into))
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

    async fn get_portfolio_with_expanded_products(
        &self,
        portfolio_id: Self::PortfolioId,
        as_of: Self::DateTime,
    ) -> Result<Option<PortfolioRecord<Self, PortfolioData>>, Self::Error> {
        let query = sqlx::query_file_as!(
            PortfolioRow,
            "queries/get_portfolio_by_id_expanded.sql",
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
    /// so the basis is actually a map of `product_id` to `weight` at that point in time.
    async fn get_portfolio_product_history(
        &self,
        portfolio_id: Self::PortfolioId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> Result<DateTimeRangeResponse<Basis<Self::ProductId>, Self::DateTime>, Self::Error> {
        let limit_p1 = (limit + 1) as i64;
        let mut rows = sqlx::query_as!(
            ValueRow::<Basis<ProductId>>,
            r#"
                select
                    valid_from as "valid_from!: crate::types::DateTime",
                    valid_until as "valid_until?: crate::types::DateTime",
                    json_group_object(product_id, weight) as "value!: sqlx::types::Json<Basis<ProductId>>"
                from
                    basis
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
