//! Interface traits for the flow trading system.
//!
//! This module contains the "ports" in the hexagonal architecture pattern.
//!
//! These traits define the contract between the domain logic and external adapters
//! (such as databases, APIs, or other services) without specifying implementation details.
//! This separation allows for easier testing and the ability to swap out infrastructure
//! components without affecting the core business logic.

use std::hash::Hash;

mod product;
pub use product::ProductRepository;

mod demand;
pub use demand::DemandRepository;

mod portfolio;
pub use portfolio::PortfolioRepository;

mod batch;
pub use batch::BatchRepository;

mod solver;
pub use solver::Solver;

/// A base trait for defining the fundamental data- and error-types.
///
/// This trait establishes the core type system used throughout the repositories.
/// Implementations define concrete types for identifiers, timestamps, and errors,
/// allowing the system to work with different backends (SQL, NoSQL, etc.).
///
/// There is no requirement or material advantage to breaking this trait across
/// multiple orthogonal traits as we have, but Rust does not allow partially
/// implementing a trait across multiple files. By breaking this trait apart,
/// this allows an implementation to keep the logic separated and ease
/// development.
pub trait Repository: Sized {
    /// The error type for underlying operations
    type Error: std::error::Error;

    /// A type suitable for expressing a timestamp
    type DateTime;

    /// A type representing a bidder id
    type BidderId: Eq + Hash;

    /// A type representing a demand id
    type DemandId: Eq + Hash;

    /// A type representing a portfolio id
    type PortfolioId: Eq + Hash;

    /// A type representing a product id
    type ProductId: Eq + Hash;
}

/// Application-level configuration and integration point.
///
/// While the Repository traits are agnostic as to the DemandData,
/// PortfolioData, and ProductData, an Application must specify concrete types.
/// Additionally, an application must provide a clock, id generators, and the
/// permissioning logic.
///
/// This trait serves as the main integration point between the generic
/// flow trading system and a specific application's requirements.
pub trait Application {
    /// An authorization context
    type Context;

    /// Application-specific data to associate to a demand curve
    type DemandData;

    /// Application-specific data to associate to a demand curve
    type PortfolioData;

    /// Application-specific data to associate to a demand curve
    type ProductData;

    /// The underlying implementation for data operations
    type Repository: DemandRepository<Self::DemandData>
        + PortfolioRepository<Self::PortfolioData>
        + ProductRepository<Self::ProductData>
        + BatchRepository<Self::Solver>;

    /// The solver to use for executing auctions
    type Solver: Solver<
            <Self::Repository as Repository>::DemandId,
            <Self::Repository as Repository>::PortfolioId,
            <Self::Repository as Repository>::ProductId,
        >;

    /// Get the application's repository
    fn database(&self) -> &Self::Repository;

    /// Get an instance of the solver
    fn solver(&self) -> Self::Solver;

    /// Get the current time
    fn now(&self) -> <Self::Repository as Repository>::DateTime;

    /// Generate an appropriate id for the provided demand data
    fn generate_demand_id(
        &self,
        data: &Self::DemandData,
    ) -> <Self::Repository as Repository>::DemandId;

    /// Generate an appropriate id for the provided portfolio data
    fn generate_portfolio_id(
        &self,
        data: &Self::PortfolioData,
    ) -> <Self::Repository as Repository>::PortfolioId;

    /// Generate an appropriate id for the provided product data
    fn generate_product_id(
        &self,
        data: &Self::ProductData,
    ) -> <Self::Repository as Repository>::ProductId;

    /// Check if the context can create new demands or portfolios.
    ///
    /// Returns the bidder ID to use for the new entity if authorized,
    /// or None if the operation is not permitted.
    fn can_create_bid(
        &self,
        context: &Self::Context,
    ) -> impl Future<Output = Option<<Self::Repository as Repository>::BidderId>> + Send;

    /// Get the list of bidders that the context is authorized to query.
    ///
    /// Returns a vector of bidder IDs that the context can access.
    /// An empty vector means no query access is granted.
    fn can_query_bid(
        &self,
        context: &Self::Context,
    ) -> impl Future<Output = Vec<<Self::Repository as Repository>::BidderId>> + Send;

    /// Check if the context can read bid data for a specific bidder.
    ///
    /// This controls access to demand and portfolio data owned by the bidder.
    fn can_read_bid(
        &self,
        context: &Self::Context,
        bidder_id: <Self::Repository as Repository>::BidderId,
    ) -> impl Future<Output = bool> + Send;

    /// Check if the context can update bid data for a specific bidder.
    ///
    /// This controls the ability to modify demands and portfolios owned by the bidder.
    fn can_update_bid(
        &self,
        context: &Self::Context,
        bidder_id: <Self::Repository as Repository>::BidderId,
    ) -> impl Future<Output = bool> + Send;

    /// Check if the context can view or query product information.
    fn can_view_products(&self, context: &Self::Context) -> impl Future<Output = bool> + Send;

    /// Check if the context can manage products (create, partition).
    fn can_manage_products(&self, context: &Self::Context) -> impl Future<Output = bool> + Send;

    /// Check if the context can execute batch auctions.
    fn can_run_batch(&self, context: &Self::Context) -> impl Future<Output = bool> + Send;
}
