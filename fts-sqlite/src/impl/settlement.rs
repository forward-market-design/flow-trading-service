use crate::{
    Db,
    types::{BidderId, DateTime, ProductId, SettlementRow},
};
use fts_core::{
    models::{
        DateTimeRangeQuery, DateTimeRangeResponse, Map, SettlementConfig, SettlementRecord,
        ValueRecord,
    },
    ports::SettlementRepository,
};

impl SettlementRepository for Db {
    async fn get_unsettled_activity<ProductMap: FromIterator<(Self::ProductId, f64)>>(
        &self,
        bidder_id: Self::BidderId,
        as_of: Self::DateTime,
    ) -> Result<(ProductMap, f64), Self::Error> {
        let trades = sqlx::query!(
            r#"
            select
                product_id as "product_id!: ProductId",
                sum(
                    trade * (
                        unixepoch(coalesce(trade_until, $2), 'subsec')
                         - unixepoch(trade_from, 'subsec')
                    )
                ) as "position!: f64",
                sum(
                    price * trade * (
                        unixepoch(coalesce(trade_until, $2), 'subsec')
                         - unixepoch(trade_from, 'subsec')
                    )
                ) as "payment!: f64"
            from
                trade_view
            where
                settled is null
            and
                price is not null
            and
                bidder_id = $1
            group by
                product_id
            "#,
            bidder_id,
            as_of,
        )
        .fetch_all(&self.reader)
        .await?;

        let mut payment = 0f64;

        let positions = trades
            .into_iter()
            .map(|record| {
                payment += record.payment;
                (record.product_id, record.position)
            })
            .collect();

        Ok((positions, payment))
    }

    async fn get_settlements(
        &self,
        bidder_id: Self::BidderId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> Result<DateTimeRangeResponse<SettlementRecord<Self>, Self::DateTime>, Self::Error> {
        let limit_p1 = (limit + 1) as i64;
        let mut results = sqlx::query_as!(
            SettlementRow,
            r#"
                select
                    as_of as "as_of!: DateTime",
                    bidder_id as "bidder_id!: BidderId",
                    json_object('position_decimals', position_decimals, 'payment_decimals', payment_decimals) as "config!: sqlx::types::Json<SettlementConfig>",
                    json_group_object(product_id, position) as "positions!: sqlx::types::Json<Map<ProductId, i64>>",
                    payment as "payment!: i64"
                from
                    settlement
                join
                    settlement_payment
                using
                    (as_of)
                join
                    settlement_position
                using
                    (as_of, bidder_id)
                where
                    bidder_id = $1
                and
                    ($2 is null or as_of >= $2)
                and
                    ($3 is null or as_of <= $3)
                group by
                    as_of, bidder_id
                order by
                    as_of desc
                limit $4
            "#,
            bidder_id,
            query.after,
            query.before,
            limit_p1,
        )
        .fetch_all(&self.reader)
        .await?;

        let more = if results.len() == limit + 1 {
            let extra = results.pop().unwrap();
            Some(DateTimeRangeQuery {
                before: Some(extra.as_of),
                after: query.after,
            })
        } else {
            None
        };

        Ok(DateTimeRangeResponse {
            results: results
                .into_iter()
                .map(|row| ValueRecord {
                    valid_from: row.as_of,
                    valid_until: None,
                    value: row.into(),
                })
                .collect(),
            more,
        })
    }

    async fn settle_activity(
        &self,
        as_of: Self::DateTime,
        config: SettlementConfig,
    ) -> Result<(), Self::Error> {
        let position_scale = 10f64.powi(config.position_decimals as i32);
        let payment_scale = 10f64.powi(config.payment_decimals as i32);

        // Stub out a new settlement.
        // This will automatically assign appropriate unsettled batches to this settlement.
        sqlx::query!(
            r#"
            insert into
                settlement (as_of, position_decimals, payment_decimals)
            values
                ($1, $2, $3)
            "#,
            as_of,
            config.position_decimals,
            config.payment_decimals
        )
        .execute(&self.writer)
        .await?;

        // Implementation plan:
        // 1. Gather batches with valid_until <= as_of, including all trades and prices
        let mut all_trades = sqlx::query_as!(
            TradeRecord,
            r#"
            select
                bidder_id as "bidder_id!: BidderId",
                product_id as "product_id!: ProductId",
                sum(
                    trade * (
                        unixepoch(trade_until, 'subsec') - unixepoch(trade_from, 'subsec')
                    )
                ) as "position!: f64",
                sum(
                    price * trade * (
                        unixepoch(trade_until, 'subsec') - unixepoch(trade_from, 'subsec')
                    )
                ) as "payment!: f64",
                0 as "rounded!: i64"
            from
                trade_view
            where
                settled = $1
            and
                price is not null
            group by
                product_id, bidder_id
            order by
                product_id
            "#,
            as_of,
        )
        .fetch_all(&self.reader)
        .await?;

        // The value is (scaled payment as float, best rounded integer of payment)
        let mut payments_by_bidder: Map<BidderId, (f64, i64)> = Default::default();

        // Since we have ordered the rows by product_id, this will chunk by product.
        // We do chunk_by_mut so that we can reuse the rows as "scratch space" for the rounding.
        for product_trades in all_trades.chunk_by_mut(|a, b| a.product_id == b.product_id) {
            let mut traded_volume = 0f64;

            let mut rounded_sold = 0i64;
            let mut rounded_bought = 0i64;

            for trade in product_trades.iter_mut() {
                // Keep track of the payments
                payments_by_bidder.entry(trade.bidder_id).or_default().0 +=
                    trade.payment * payment_scale;

                // scale the position to the integral basis
                trade.position *= position_scale;

                // update the trade volume (so we can establish a rounding target)
                traded_volume += trade.position.abs();

                // initially, we round towards zero and store the rounded value as an integer
                // if we have "leftovers" from the rounded, we track it as the position
                let trunc = trade.position.trunc();
                trade.position -= trunc; // this is the outstanding position not covered by the current round
                trade.rounded = trunc as i64; // this is the current rounded value

                // finally, we need a "current" set of totals to inform how we adjust rounding
                rounded_sold -= 0i64.min(trade.rounded);
                rounded_bought += 0i64.max(trade.rounded);
            }

            // We separately round the volumes to nearest amount, taking the smaller of the two as the target
            // It should be sell_volume == buy_volume, but if we wind up near 0.5 +/- eps, this is a safe way to go.
            let target_volume = (traded_volume * 0.5).round_ties_even() as i64;

            // We now sort by outstanding position
            product_trades.sort_unstable_by(|x, y| x.position.total_cmp(&y.position));

            // We update the sell trades
            for trade in product_trades
                .iter_mut()
                .take((target_volume - rounded_sold) as usize)
            {
                // TODO: prove we do not need this assertion
                assert!(trade.position < 0.0);
                trade.position += 1.0;
                trade.rounded -= 1;
            }

            // We update the buy trades
            for trade in product_trades
                .iter_mut()
                .rev()
                .take((target_volume - rounded_bought) as usize)
            {
                // TODO: prove we do not need this assertion
                assert!(trade.position > 0.0);
                trade.position -= 1.0;
                trade.rounded += 1;
            }
        }

        // With the positions sorted, we now turn to payments.
        // This proceeds quite similarly as to the above, the only
        // differences being the global/local nature of the quantities

        let mut total_payment = 0f64;
        let mut rounded_sold = 0i64;
        let mut rounded_bought = 0i64;

        for (paid, rounded) in payments_by_bidder.values_mut() {
            total_payment += paid.abs();

            let trunc = paid.trunc();
            *paid -= trunc;
            *rounded = trunc as i64;

            rounded_sold -= 0i64.min(*rounded);
            rounded_bought += 0i64.max(*rounded);
        }

        let target_payment = (total_payment * 0.5).round_ties_even() as i64;

        // Next, we sort by outstanding rounding error
        payments_by_bidder.sort_unstable_by(|_, x, _, y| x.0.total_cmp(&y.0));

        // Now it's the same as before:
        for (delta, rounded) in payments_by_bidder
            .values_mut()
            .take((target_payment - rounded_sold) as usize)
        {
            assert!(*delta < 0.0);
            *delta += 1.0;
            *rounded -= 1;
        }
        for (delta, rounded) in payments_by_bidder
            .values_mut()
            .rev()
            .take((target_payment - rounded_bought) as usize)
        {
            assert!(*delta > 0.0);
            *delta -= 1.0;
            *rounded += 1;
        }

        // 3. Write results. Because inserting lots of records is awkward, we update the json blobs
        //    and rely on a database trigger to explode the blobs into individual records.
        {
            let positions = sqlx::types::Json(all_trades);
            let payments = sqlx::types::Json(payments_by_bidder);
            sqlx::query!(
                r#"
                update
                    settlement
                set
                    positions = jsonb($2),
                    payments = jsonb($3)
                where
                    as_of = $1
                "#,
                as_of,
                positions,
                payments,
            )
            .execute(&self.writer)
            .await?;
        };

        Ok(())
    }
}

#[derive(serde::Serialize)]
struct TradeRecord {
    bidder_id: BidderId,
    product_id: ProductId,
    #[serde(skip_serializing)]
    position: f64,
    #[serde(skip_serializing)]
    payment: f64,
    rounded: i64,
}
