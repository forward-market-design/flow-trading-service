use crate::models::{DateTimeRangeQuery, DateTimeRangeResponse, ValueRecord};

/// Repository interface for batch auction execution and outcome retrieval.
///
/// This trait provides methods for running batch auctions at specific timestamps
/// and retrieving the resulting allocations and prices.
pub trait BatchRepository<T: super::Solver<Self::DemandId, Self::PortfolioId, Self::ProductId>>:
    super::Repository
{
    /// Execute a batch auction for a specific timestamp.
    ///
    /// Gather all the portfolios and demand curves for the requested time
    /// and solve the corresponding auction using `solver`.
    ///
    /// # Returns
    ///
    /// - Ok(Ok(())) if the batch completed successfully
    /// - Ok(Err(solver_error)) if the solver failed
    /// - Err(repository_error) if there is some other error
    fn run_batch(
        &self,
        timestamp: Self::DateTime,
        solver: T,
        state: T::State,
    ) -> impl Future<Output = Result<Result<(), T::Error>, Self::Error>> + Send;

    /// Retrieve historical batch outcomes for a portfolio.
    ///
    /// # Returns
    ///
    /// A paginated response containing the portfolio's allocations from past batches.
    fn get_portfolio_outcomes(
        &self,
        portfolio_id: Self::PortfolioId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> impl Future<
        Output = Result<
            DateTimeRangeResponse<ValueRecord<Self::DateTime, T::PortfolioOutcome>, Self::DateTime>,
            Self::Error,
        >,
    > + Send;

    /// Retrieve historical batch outcomes for a product.
    ///
    /// # Returns
    ///
    /// A paginated response containing the product's clearing prices from past batches.
    fn get_product_outcomes(
        &self,
        product_id: Self::ProductId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> impl Future<
        Output = Result<
            DateTimeRangeResponse<ValueRecord<Self::DateTime, T::ProductOutcome>, Self::DateTime>,
            Self::Error,
        >,
    > + Send;
}
