use crate::models::{Basis, DateTimeRangeQuery, DateTimeRangeResponse, PortfolioRecord, Weights};

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
        demand: Weights<Self::DemandId>,
        basis: Basis<Self::ProductId>,
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<PortfolioRecord<Self, PortfolioData>, Self::Error>> + Send;

    /// Update a portfolio's demand group.
    fn update_portfolio_demand(
        &self,
        portfolio_id: Self::PortfolioId,
        demand: Weights<Self::DemandId>,
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<Option<PortfolioRecord<Self, PortfolioData>>, Self::Error>> + Send;

    /// Update a portfolio's product group
    fn update_portfolio_basis(
        &self,
        portfolio_id: Self::PortfolioId,
        basis: Basis<Self::ProductId>,
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<Option<PortfolioRecord<Self, PortfolioData>>, Self::Error>> + Send;

    /// Update both the demand- and product- groups at once.
    fn update_portfolio(
        &self,
        portfolio_id: Self::PortfolioId,
        demand: Weights<Self::DemandId>,
        basis: Basis<Self::ProductId>,
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<Option<PortfolioRecord<Self, PortfolioData>>, Self::Error>> + Send;

    /// Retrieve a portfolio at a specific point in time.
    fn get_portfolio(
        &self,
        portfolio_id: Self::PortfolioId,
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<Option<PortfolioRecord<Self, PortfolioData>>, Self::Error>> + Send;

    /// Retrieve a portfolio at a specific point in time, with the product group expanded in the
    /// contemporary product basis.
    fn get_portfolio_with_expanded_products(
        &self,
        portfolio_id: Self::PortfolioId,
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<Option<PortfolioRecord<Self, PortfolioData>>, Self::Error>> + Send;

    /// Query all the portfolios with non-empty groups associated to `bidder_id`.
    ///
    /// # Returns
    ///
    /// A vector of "active" (as of the time of querying) portfolio records.
    fn query_portfolio(
        &self,
        bidder_ids: &[Self::BidderId],
    ) -> impl Future<Output = Result<Vec<PortfolioRecord<Self, PortfolioData>>, Self::Error>> + Send;

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
            DateTimeRangeResponse<Weights<Self::DemandId>, Self::DateTime>,
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
        Output = Result<DateTimeRangeResponse<Basis<Self::ProductId>, Self::DateTime>, Self::Error>,
    > + Send;
}
