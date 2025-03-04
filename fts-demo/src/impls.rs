// These implement the respective *Repository traits
mod auction;
mod auth;
mod cost;
mod product;
mod submission;

// This trait does nothing, other than prove we have everything we need.
impl fts_core::ports::MarketRepository for crate::db::Database {}
