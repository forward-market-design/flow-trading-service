use crate::models::{
    AuctionOutcome, DateTimeRangeQuery, DateTimeRangeResponse, ProductId, ProductQueryResponse,
    ProductRecord,
};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;
use time::OffsetDateTime;

/// Repository trait for product-related operations.
///
/// This trait provides functionality for managing products in the trading system.
/// Products are the fundamental tradable entities in the system, which can be
/// referenced by authorization portfolios and are constrained to net-zero trade
/// in each auction.
///
/// Implementations provide methods for defining new products, querying existing
/// products, and retrieving auction outcomes for specific products.
pub trait ProductRepository: Clone + Sized + Send + Sync + 'static {
    /// The error type returned by this repository's operations
    type Error: std::error::Error + Send + Sync + 'static;

    /// An implementation must provide a type describing the products
    type ProductData: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// An implementation must also provide a query type
    type ProductQuery: Serialize + DeserializeOwned + Send + Sync + 'static;

    /// Define new products
    fn define_products(
        &self,
        products: impl Iterator<Item = Self::ProductData> + Send,
        timestamp: OffsetDateTime,
    ) -> impl Future<Output = Result<Vec<ProductId>, Self::Error>> + Send;

    /// View a specific product by its id
    fn view_product(
        &self,
        product_id: ProductId,
    ) -> impl Future<Output = Result<Option<ProductRecord<Self::ProductData>>, Self::Error>> + Send;

    /// Search for products using a query
    fn query_products(
        &self,
        query: Self::ProductQuery,
        limit: usize,
    ) -> impl Future<
        Output = Result<
            ProductQueryResponse<ProductRecord<Self::ProductData>, Self::ProductQuery>,
            Self::Error,
        >,
    > + Send;

    /// Retrieve any posted results
    fn get_outcomes(
        &self,
        product_id: ProductId,
        query: DateTimeRangeQuery,
        limit: usize,
    ) -> impl Future<Output = Result<DateTimeRangeResponse<AuctionOutcome<()>>, Self::Error>> + Send;
}
