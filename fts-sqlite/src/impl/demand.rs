use crate::{
    Db,
    types::{BidderId, DateTime, DemandId, DemandRow, PortfolioId, ValueRow},
};
use fts_core::{
    models::{
        DateTimeRangeQuery, DateTimeRangeResponse, DemandCurve, DemandCurveDto, DemandRecord,
        PortfolioGroup, ValueRecord,
    },
    ports::DemandRepository,
};

impl<DemandData: Send + Unpin + serde::Serialize + serde::de::DeserializeOwned>
    DemandRepository<DemandData> for Db
{
    async fn get_demand_bidder_id(
        &self,
        demand_id: Self::DemandId,
    ) -> Result<Option<Self::BidderId>, Self::Error> {
        sqlx::query_scalar!(
            r#"
            select
                bidder_id as "id!: BidderId"
            from
                demand
            where
                id = $1
            "#,
            demand_id
        )
        .fetch_optional(&self.reader)
        .await
    }

    async fn query_demand(
        &self,
        bidder_ids: &[Self::BidderId],
    ) -> Result<Vec<DemandRecord<Self, DemandData>>, Self::Error> {
        if bidder_ids.len() == 0 {
            Ok(Vec::new())
        } else {
            let bidder_ids = sqlx::types::Json(bidder_ids);
            let query = sqlx::query_as!(
                DemandRow,
                r#"
                select
                    demand.id as "id!: DemandId",
                    as_of as "valid_from!: DateTime",
                    null as "valid_until?: DateTime",
                    bidder_id as "bidder_id!: BidderId",
                    json(app_data) as "app_data!: sqlx::types::Json<DemandData>",
                    json(curve_data) as "curve_data?: sqlx::types::Json<DemandCurveDto>",
                    null as "portfolio_group?: sqlx::types::Json<PortfolioGroup<PortfolioId>>"
                from
                    demand
                join
                    json_each($1) as bidder_ids
                on
                    demand.bidder_id = bidder_ids.atom
                where
                    curve_data is not null
                "#,
                bidder_ids
            )
            .fetch_all(&self.reader)
            .await?;

            Ok(query.into_iter().map(Into::into).collect())
        }
    }

    async fn create_demand(
        &self,
        demand_id: Self::DemandId,
        bidder_id: Self::BidderId,
        app_data: DemandData,
        curve_data: Option<DemandCurve>,
        as_of: Self::DateTime,
    ) -> Result<DemandRecord<Self, DemandData>, Self::Error> {
        let app_data = sqlx::types::Json(app_data);
        // Important: If curve_data is None, we insert NULL into the database
        // Else, this propagates into a [0] value in the JSONB column
        let curve_data = curve_data.map(|x| sqlx::types::Json(x));
        let demand = sqlx::query_as!(
            DemandRow::<DemandData>,
            r#"
            insert into
                demand (id, as_of, bidder_id, app_data, curve_data)
            values
                ($1, $2, $3, jsonb($4), jsonb($5))
            returning
                id as "id!: DemandId",
                as_of as "valid_from!: DateTime",
                null as "valid_until?: DateTime",
                bidder_id as "bidder_id!: BidderId",
                json(app_data) as "app_data!: sqlx::types::Json<DemandData>",
                json(curve_data) as "curve_data?: sqlx::types::Json<DemandCurveDto>",
                null as "portfolio_group?: sqlx::types::Json<PortfolioGroup<PortfolioId>>"
            "#,
            demand_id,
            as_of,
            bidder_id,
            app_data,
            curve_data,
        )
        .fetch_one(&self.writer)
        .await?;
        Ok(demand.into())
    }

    async fn update_demand(
        &self,
        demand_id: Self::DemandId,
        curve_data: Option<DemandCurve>,
        as_of: Self::DateTime,
    ) -> Result<Option<DemandRecord<Self, DemandData>>, Self::Error> {
        let curve_data = curve_data.map(|x| sqlx::types::Json(x));
        let demand = sqlx::query_as!(
            DemandRow::<DemandData>,
            r#"
            update
                demand
            set
                as_of = $2,
                curve_data = jsonb($3)
            where
                id = $1
            returning
                id as "id!: DemandId",
                as_of as "valid_from!: DateTime",
                null as "valid_until?: DateTime",
                bidder_id as "bidder_id!: BidderId",
                json(app_data) as "app_data!: sqlx::types::Json<DemandData>",
                json(curve_data) as "curve_data?: sqlx::types::Json<DemandCurveDto>",
                null as "portfolio_group?: sqlx::types::Json<PortfolioGroup<PortfolioId>>"
            "#,
            demand_id,
            as_of,
            curve_data,
        )
        .fetch_optional(&self.writer)
        .await?
        .map(Into::into);
        Ok(demand)
    }

    async fn get_demand(
        &self,
        demand_id: Self::DemandId,
        as_of: Self::DateTime,
    ) -> Result<Option<DemandRecord<Self, DemandData>>, Self::Error> {
        let query =
            sqlx::query_file_as!(DemandRow, "queries/get_demand_by_id.sql", demand_id, as_of)
                .fetch_optional(&self.reader)
                .await?;

        Ok(query.map(Into::into))
    }

    async fn get_demand_curve_history(
        &self,
        demand_id: Self::DemandId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> Result<DateTimeRangeResponse<DemandCurve, Self::DateTime>, Self::Error> {
        let limit_p1 = (limit + 1) as i64;
        let mut rows = sqlx::query_as!(
            ValueRow::<DemandCurveDto>,
            r#"
                select
                    valid_from as "valid_from!: DateTime",
                    valid_until as "valid_until?: DateTime",
                    json(value) as "value!: sqlx::types::Json<DemandCurveDto>"
                from
                    curve_data
                where
                    demand_id = $1
                and
                    ($2 is null or valid_from >= $2)
                and
                    ($3 is null or valid_until is null or valid_until < $3)
                and
                    value is not null
                order by
                    valid_from desc
                limit $4
            "#,
            demand_id,
            query.after,
            query.before,
            limit_p1, // +1 to check if there are more results
        )
        .fetch_all(&self.reader)
        .await?;

        // We paginate by adding 1 to the limit, popping the result of, and
        // using it to adjust the query object
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
            results: rows
                .into_iter()
                .map(
                    |ValueRow {
                         valid_from,
                         valid_until,
                         value,
                     }| ValueRecord {
                        valid_from,
                        valid_until,
                        value: unsafe { DemandCurve::new_unchecked(value.0) },
                        // SAFETY: this is only being called when deserializing a SQL query, and we ensure curves
                        //         are valid going into the database.
                    },
                )
                .collect(),
            more,
        })
    }
}
