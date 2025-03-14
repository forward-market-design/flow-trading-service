// #[cfg(feature = "admm")]
// pub mod admm;

/// Implementation using the Clarabel interior point solver
#[cfg(feature = "clarabel")]
pub mod clarabel;

/// Implementation using the OSQP operator splitting solver
#[cfg(feature = "osqp")]
pub mod osqp;
