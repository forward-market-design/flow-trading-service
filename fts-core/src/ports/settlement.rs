/*
Implementation notes:

* We need an "unsettled" abstraction. This can be used to provide inputs for the rounding.
* Question: is unsettled by product, or by portfolio? (I think it's by portfolio?)
* No. It *must* be by product, because portfolios are no longer immutable.
*

*/

/// Repository interface for settlements of trade and payments.
///
/// Batches are about individual auctions, which resolve trades on portfolios
/// and prices on products. Notably, this does not immediately produce positions
/// on individual products (though it is straightforward to multiply a portfolio's
/// traded amount by the amount of an individual product it contains).
///
/// As a point of inconsistency, the batches *do* report each portfolio's
/// effective price (even though it is an indirect artifact of the outcome).
/// In this case, the portfolio price is trivial to compute, introduces only
/// an additional scalar per portfolio to store, and is useful enough that we
/// ask the solver to include this in its output. Conversely, the "rolled down"
/// positions from each portfolio may span millions of products, introducing a
/// significant memory burden and are not reported by the solver.
///
/// Nevertheless, it is obviously necessary to understand a bidder's net position
/// across the products they bid upon. Settlement is decoupled from batch
/// execution, but may occur as often as desired. The act of settlement combines
/// the trades (and payments) from each unsettled batch, employs a simple
/// financial rounding routine, then reports the combined net positions for each
/// bidder x product and net payment for each bidder. Settling multiple batches
/// at once minimizes losses due to rounding.
pub trait SettlementRepository: super::Repository {
    /// Find the "raw", unsettled activity (product trades and bidder payments).
    ///
    /// If `bidders` is None,
    fn unsettled_activity<
        BidderMap: FromIterator<(Self::BidderId, f64)>,
        ProductMap: FromIterator<(Self::ProductId, f64)>,
    >(
        &self,
        bidders: Option<&[Self::BidderId]>,
    ) -> impl Future<Output = Result<(BidderMap, ProductMap), Self::Error>> + Send;

    //fn settle_batches(&self, as_of: Self::DateTime, position_decimals: u8, cash_decimals: u8);
}
