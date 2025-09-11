use crate::{
    Db,
    types::{BidderId, ProductId},
};
use fts_core::{
    models::{
        Amount, DateTimeRangeQuery, DateTimeRangeResponse, Map, SettlementConfig, SettlementRecord,
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
                settlement_id is null
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

    async fn get_settlements<ProductMap: FromIterator<(Self::ProductId, Amount)>>(
        &self,
        bidder_id: Self::BidderId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> Result<
        DateTimeRangeResponse<SettlementRecord<Self, ProductMap>, Self::DateTime>,
        Self::Error,
    > {
        todo!()
    }

    async fn settle_activity<ProductMap: FromIterator<(Self::ProductId, Amount)>>(
        &self,
        as_of: Self::DateTime,
        config: SettlementConfig,
    ) -> Result<(), Self::Error> {
        let position_scale = 10f64.powi(config.position_decimals as i32);
        let payment_scale = 10f64.powi(config.payment_decimals as i32);

        // Implementation plan:
        // 1. Gather batches with valid_until <= as_of, including all trades and prices
        let mut all_trades = sqlx::query!(
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
                settlement_id is null
            and
                trade_until <= $1
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

        let mut bidders: Map<BidderId, (f64, i64)> = Default::default();

        for product_trades in all_trades.chunk_by_mut(|a, b| a.product_id == b.product_id) {
            let mut sell_volume = 0f64;
            let mut buy_volume = 0f64;

            let mut sell_total = 0i64;
            let mut buy_total = 0i64;

            for trade in product_trades.iter_mut() {
                // Keep track of the payments
                bidders.entry(trade.bidder_id).or_default().0 += trade.payment * payment_scale;

                // scale the position to the integral basis
                trade.position *= position_scale;

                // update the trade volume (so we can establish a rounding target)
                sell_volume -= 0f64.min(trade.position);
                buy_volume += 0f64.max(trade.position);

                // initially, we round towards zero and store the rounded value as an integer
                // if we have "leftovers" from the rounded, we track it as the position
                let trunc = trade.position.trunc();
                trade.position -= trunc; // this is the outstanding position not covered by the current round
                trade.rounded = trunc as i64; // this is the current rounded value

                // finally, we need a "current" set of totals to inform how we adjust rounding
                sell_total -= 0i64.min(trade.rounded);
                buy_total += 0i64.max(trade.rounded);
            }

            // We separately round the volumes to nearest amount, taking the smaller of the two as the target
            // It should be sell_volume == buy_volume, but if we wind up near 0.5 +/- eps, this is a safe way to go.
            let target_volume = sell_volume.round().min(buy_volume.round()) as i64;

            // We now sort by outstanding position
            product_trades.sort_unstable_by(|x, y| x.position.total_cmp(&y.position));

            // We update the sell trades
            for trade in product_trades
                .iter_mut()
                .take((target_volume - sell_total) as usize)
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
                .take((target_volume - buy_total) as usize)
            {
                // TODO: prove we do not need this assertion
                assert!(trade.position > 0.0);
                trade.position -= 1.0;
                trade.rounded += 1;
            }
        }

        // With the positions sorted, we now turn to payments.
        // This proceeds quite similarly as to the above.

        let mut raw_sell_payment = 0f64;
        let mut raw_buy_payment = 0f64;
        let mut rounded_sell_payment = 0i64;
        let mut rounded_buy_payment = 0i64;

        for payment in bidders.values_mut() {
            raw_sell_payment -= 0f64.min(payment.0);
            raw_buy_payment += 0f64.max(payment.0);

            let trunc = payment.0.trunc();
            payment.0 -= trunc;
            payment.1 = trunc as i64;

            rounded_sell_payment -= 0i64.min(payment.1);
            rounded_buy_payment += 0i64.max(payment.1);
        }

        let target_payment = raw_sell_payment.round().min(raw_buy_payment.round()) as i64;

        // Next, we sort by outstanding rounding error
        bidders.sort_unstable_by(|_, x, _, y| x.0.total_cmp(&y.0));

        // Now it's the same as before:
        for value in bidders
            .values_mut()
            .take((target_payment - rounded_sell_payment) as usize)
        {
            assert!(value.0 < 0.0);
            value.0 += 1.0;
            value.1 -= 1;
        }
        for value in bidders
            .values_mut()
            .rev()
            .take((target_payment - rounded_buy_payment) as usize)
        {
            assert!(value.0 > 0.0);
            value.0 -= 1.0;
            value.1 += 1;
        }

        // 3. Write results

        // `bidders` is { [bidder_id]: i64 }
        // and `all_trade` is { bidder_id, product_id, rounded, ...}.
        // This is a lot of data to send back to SQL:
        // * we probably don't want the overhead of a big INSERT clause,
        //   e.g. we're better of packaging into some JSON blobs.
        todo!()
    }
}
