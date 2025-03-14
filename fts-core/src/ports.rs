mod auction;
mod auth;
mod cost;
mod product;
mod submission;

pub use auction::AuctionRepository;
pub use auth::{AuthFailure, AuthRepository};
pub use cost::{CostFailure, CostRepository};
pub use product::ProductRepository;
pub use submission::{
    SubmissionAuthDto, SubmissionCostDto, SubmissionDto, SubmissionFailure, SubmissionRepository,
};

/// A marker trait that combines all repository functionality
///
/// This trait serves as a composition point for all repository traits,
/// allowing consumers to depend on a single trait that provides all repository capabilities.
pub trait MarketRepository: AuctionRepository {}
