use crate::Db;
use fts_core::ports::SettlementRepository;

impl SettlementRepository for Db {
    async fn unsettled_activity<
        BidderMap: FromIterator<(Self::BidderId, f64)>,
        ProductMap: FromIterator<(Self::ProductId, f64)>,
    >(
        &self,
        _bidders: Option<&[Self::BidderId]>,
    ) -> Result<(BidderMap, ProductMap), Self::Error> {
        // let positions = sqlx::query!(
        //     PortfolioRow,
        //     r#"
        //         select
        //             bidder_id,
        //             product_id,
        //             sum(trade * (coalesce(juliaday(valid_until), julianday('now')) - julianday(valid_from))
        //         from
        //             batch
        //         join
        //             trade_view
        //         on
        //             batch.id = trade_view.batch_id
        //         join
        //             product_tree
        //         join
        //             json_each($1) as bidder_ids
        //         on
        //             trade_view.bidder_id = bidder_ids.atom
        //         where
        //             batch.settlement_id is null
        //         group by
        //             bidder_id, product_id
        //         "#,
        //     bidder_ids
        // )
        // .fetch_all(&self.reader)
        // .await?;

        todo!();
    }
}
