use crate::{
    Db,
    types::{BidderId, DateTime, DemandHistoryRow, DemandId, DemandRow, PortfolioId},
};
use fts_core::{
    models::{
        DateTimeRangeQuery, DateTimeRangeResponse, DemandCurve, DemandCurveDto, DemandRecord, Map,
        ValueRecord,
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
        as_of: Self::DateTime,
    ) -> Result<Vec<Self::DemandId>, Self::Error> {
        if bidder_ids.len() == 0 {
            Ok(Vec::new())
        } else {
            let bidder_ids = sqlx::types::Json(bidder_ids);
            sqlx::query_scalar!(
                r#"
                select
                    demand.id as "id!: DemandId"
                from
                    demand
                join
                    curve_data
                on
                    demand.id = curve_data.demand_id
                join
                    json_each($1) as bidder_ids
                on
                    demand.bidder_id = bidder_ids.atom
                where
                    curve_data.value is not null
                and
                    valid_from <= $2
                and
                    ($2 < valid_until or valid_until is null) 
                "#,
                bidder_ids,
                as_of,
            )
            .fetch_all(&self.reader)
            .await
        }
    }

    async fn create_demand(
        &self,
        demand_id: Self::DemandId,
        bidder_id: Self::BidderId,
        app_data: DemandData,
        curve_data: Option<DemandCurve>,
        as_of: Self::DateTime,
    ) -> Result<(), Self::Error> {
        let app_data = sqlx::types::Json(app_data);
        // Important: If curve_data is None, we insert NULL into the database
        // Else, this propagates into a [0] value in the JSONB column
        let curve_data = curve_data.map(|x| sqlx::types::Json(x));
        sqlx::query!(
            r#"
            insert into
                demand (id, as_of, bidder_id, app_data, curve_data)
            values
                (?, ?, ?, jsonb(?), jsonb(?))
            "#,
            demand_id,
            as_of,
            bidder_id,
            app_data,
            curve_data,
        )
        .execute(&self.writer)
        .await?;
        Ok(())
    }

    async fn update_demand(
        &self,
        demand_id: Self::DemandId,
        curve_data: Option<DemandCurve>,
        as_of: Self::DateTime,
    ) -> Result<bool, Self::Error> {
        let curve_data = curve_data.map(|x| sqlx::types::Json(x));
        let query = sqlx::query!(
            r#"
            update
                demand
            set
                as_of = $2,
                curve_data = jsonb($3)
            where
                id = $1
            "#,
            demand_id,
            as_of,
            curve_data,
        )
        .execute(&self.writer)
        .await?;

        Ok(query.rows_affected() > 0)
    }

    async fn get_demand(
        &self,
        demand_id: Self::DemandId,
        as_of: Self::DateTime,
    ) -> Result<
        Option<
            DemandRecord<
                Self::DateTime,
                Self::BidderId,
                Self::DemandId,
                Self::PortfolioId,
                DemandData,
            >,
        >,
        Self::Error,
    > {
        let query =
            sqlx::query_file_as!(DemandRow, "queries/get_demand_by_id.sql", demand_id, as_of)
                .fetch_optional(&self.reader)
                .await?;

        Ok(query.map(|row| DemandRecord {
            id: demand_id,
            as_of,
            bidder_id: row.bidder_id,
            app_data: row.app_data.0,
            curve_data: row
                .curve_data
                // SAFETY: `curve_data` was necessarily serialized from a valid curve, so we can safely skip the validation
                .map(|data| unsafe { DemandCurve::new_unchecked(data.0) }),
            portfolio_group: row.portfolio_group.map(|data| data.0).unwrap_or_default(),
        }))
    }

    async fn get_demand_history(
        &self,
        demand_id: Self::DemandId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> Result<
        DateTimeRangeResponse<ValueRecord<Self::DateTime, DemandCurve>, Self::DateTime>,
        Self::Error,
    > {
        let limit_p1 = (limit + 1) as i64;
        let mut rows = sqlx::query_as!(
            DemandHistoryRow,
            r#"
                select
                    valid_from as "valid_from!: DateTime",
                    valid_until as "valid_until?: DateTime",
                    json(value) as "curve_data!: sqlx::types::Json<DemandCurveDto>"
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
            results: rows.into_iter().map(Into::into).collect(),
            more,
        })
    }
}
