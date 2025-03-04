use crate::{DateTime, db};
use fts_core::{
    models::{
        AuctionOutcome, DateTimeRangeQuery, DateTimeRangeResponse, Outcome, ProductId,
        ProductQueryResponse, ProductRecord,
    },
    ports::ProductRepository,
};
use rusqlite::OptionalExtension as _;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ProductData {
    pub kind: String,
    #[serde(with = "time::serde::rfc3339")]
    pub from: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub thru: OffsetDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductQuery {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub before: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub after: Option<OffsetDateTime>,
}

impl ProductRepository for db::Database {
    type Error = db::Error;
    type ProductData = ProductData;
    type ProductQuery = ProductQuery;

    async fn define_products(
        &self,
        products: impl Iterator<Item = ProductData>,
        timestamp: &OffsetDateTime,
    ) -> Result<Vec<ProductId>, Self::Error> {
        let mut ctx = self.connect(true)?;
        let tx = ctx.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let product_ids = {
            // sqlite triggers automatically handle any product tree maintenance
            let mut insert_stmt = tx.prepare(
                r#"
                insert into product (id, kind, "from", thru, timestamp) values (?, ?, ?, ?, ?)
                "#,
            )?;

            let ids = products.map(|product| -> Result<ProductId, db::Error> {
                // Create the new product id
                let id = Uuid::new_v4();
                insert_stmt.execute((
                    &id,
                    &product.kind,
                    DateTime::from(product.from),
                    DateTime::from(product.thru),
                    DateTime::from(timestamp),
                ))?;

                Ok(id.into())
            });

            ids.collect::<Result<Vec<_>, _>>()?
        };

        tx.commit()?;

        Ok(product_ids)
    }

    async fn query_products(
        &self,
        query: &Self::ProductQuery,
        limit: usize,
    ) -> Result<
        ProductQueryResponse<ProductRecord<Self::ProductData>, Self::ProductQuery>,
        Self::Error,
    > {
        let ctx = self.connect(false)?;
        let mut stmt = ctx.prepare(
            r#"
            select
                id, kind, "from", thru
            from
                product
            where
                (?1 is null or ?1 = kind)
            and
                (?2 is null or "from" <= ?2)
            and
                (?3 is null or thru >= ?3)
            order by
                "from" asc, thru asc, kind
            limit
                ?4
            "#,
        )?;

        let mut results = stmt
            .query_and_then(
                (
                    &query.kind,
                    query.before.map(DateTime::from),
                    query.after.map(DateTime::from),
                    limit + 1,
                ),
                product_from_row,
            )?
            .collect::<Result<Vec<_>, _>>()?;

        let more = if results.len() == limit + 1 {
            // Safe, since limit + 1 >= 1.
            let extra = results.pop().unwrap();
            // Note that due to multiple kinds, the next query might
            // pick up some duplicates. TODO
            // Also TODO: what about product tree nesting?
            Some(ProductQuery {
                kind: query.kind.clone(),
                before: query.before.clone(),
                after: Some(extra.data.from),
            })
        } else {
            None
        };

        Ok(ProductQueryResponse { results, more })
    }

    async fn view_product(
        &self,
        product_id: &ProductId,
    ) -> Result<Option<ProductRecord<Self::ProductData>>, Self::Error> {
        let ctx = self.connect(false)?;
        let query = ctx
            .query_row_and_then(
                r#"select id, kind, "from", thru from product where id = ?"#,
                (product_id.deref(),),
                product_from_row,
            )
            .optional()?;

        Ok(query)
    }

    async fn get_outcomes(
        &self,
        product_id: &ProductId,
        query: &DateTimeRangeQuery,
        limit: usize,
    ) -> Result<DateTimeRangeResponse<AuctionOutcome<()>>, Self::Error> {
        let ctx = self.connect(false)?;
        let mut stmt = ctx.prepare(
            r#"
            select
                "from",
                "thru",
                price,
                trade
            from
                product_outcome
            join
                auction
            on
                product_outcome.auction_id = auction.id
            where
                product_id = ?1
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
                    product_id.deref(),
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

        Ok(DateTimeRangeResponse { results, more })
    }
}

fn product_from_row(row: &rusqlite::Row) -> rusqlite::Result<ProductRecord<ProductData>> {
    Ok(ProductRecord {
        id: row.get::<&str, Uuid>("id")?.into(),
        data: ProductData {
            kind: row.get("kind")?,
            from: row.get::<&str, DateTime>("from")?.into(),
            thru: row.get::<&str, DateTime>("thru")?.into(),
        },
    })
}
