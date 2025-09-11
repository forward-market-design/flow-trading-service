use crate::{Db, types::ProductId};
use fts_core::{
    models::{
        Amount, DateTimeRangeQuery, DateTimeRangeResponse, SettlementConfig, SettlementRecord,
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
        decimals: SettlementConfig,
    ) -> Result<SettlementRecord<Self, ProductMap>, Self::Error> {
        todo!()
    }
}
