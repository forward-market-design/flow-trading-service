use crate::models::{
    DateTimeRangeQuery, DateTimeRangeResponse, DemandGroup, PortfolioRecord, ProductGroup,
};

/// Repository interface for portfolio CRUD operations and history tracking.
///
/// Portfolios aggregate demands and associate them with tradeable products.
/// This trait provides methods for creating, updating, and querying portfolios,
/// as well as tracking their historical changes.
///
/// This trait is parameterized by a generic data type, allowing an application
/// to colocate write-once data alongside the relevant record.
pub trait PortfolioRepository<PortfolioData>: super::Repository {
    /// Get the bidder id associated to the portfolio
    fn get_portfolio_bidder_id(
        &self,
        portfolio_id: Self::PortfolioId,
    ) -> impl Future<Output = Result<Option<Self::BidderId>, Self::Error>> + Send;

    /// Create a new portfolio with initial demand and product associations.
    fn create_portfolio(
        &self,
        portfolio_id: Self::PortfolioId,
        bidder_id: Self::BidderId,
        app_data: PortfolioData,
        demand_group: DemandGroup<Self::DemandId>,
        product_group: ProductGroup<Self::ProductId>,
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Update a portfolio's demand and/or product associations.
    ///
    /// Optionally updates the demand curves and products the portfolio is associated to.
    /// (Provide a `None` value to not update the respective data.)
    ///
    /// # Returns
    ///
    /// - Ok(true) if successful
    /// - Ok(false) if portfolio does not exist
    /// - Err otherwise
    fn update_portfolio(
        &self,
        portfolio_id: Self::PortfolioId,
        demand_group: Option<DemandGroup<Self::DemandId>>,
        product_group: Option<ProductGroup<Self::ProductId>>,
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;

    /// Retrieve a portfolio at a specific point in time.
    ///
    /// Retrieve the requested portfolio and associated demand curves, returning Option::None if it does not exist.
    /// The portfolio's product_group will be *fully expanded* in the contemporary product basis (in the event that
    /// any products have been partitioned since the initial creation).
    fn get_portfolio(
        &self,
        portfolio_id: Self::PortfolioId,
        as_of: Self::DateTime,
    ) -> impl Future<
        Output = Result<
            Option<
                PortfolioRecord<
                    Self::DateTime,
                    Self::BidderId,
                    Self::PortfolioId,
                    Self::DemandId,
                    Self::ProductId,
                    PortfolioData,
                >,
            >,
            Self::Error,
        >,
    > + Send;

    /// Query all the portfolios with non-empty groups associated to `bidder_id`
    /// as-of the specified time.
    ///
    /// # Returns
    ///
    /// A vector of portfolio IDs that match the query criteria.
    fn query_portfolio(
        &self,
        bidder_ids: &[Self::BidderId],
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<Vec<Self::PortfolioId>, Self::Error>> + Send;

    /// Retrieve the history of demand group changes for a portfolio.
    ///
    /// # Returns
    ///
    /// A paginated response containing historical demand group records.
    fn get_portfolio_demand_history(
        &self,
        portfolio_id: Self::PortfolioId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> impl Future<
        Output = Result<
            DateTimeRangeResponse<DemandGroup<Self::DemandId>, Self::DateTime>,
            Self::Error,
        >,
    > + Send;

    /// Retrieve the history of product group changes for a portfolio.
    ///
    /// # Returns
    ///
    /// A paginated response containing historical product group records.
    fn get_portfolio_product_history(
        &self,
        portfolio_id: Self::PortfolioId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> impl Future<
        Output = Result<
            DateTimeRangeResponse<ProductGroup<Self::ProductId>, Self::DateTime>,
            Self::Error,
        >,
    > + Send;
}
