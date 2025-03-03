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

// The "marker" trait that is used every and implies implementation of all the above
pub trait MarketRepository: AuctionRepository {}
