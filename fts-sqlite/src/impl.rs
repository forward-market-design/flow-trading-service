//! Repository trait implementations for the SQLite database.
//!
//! This module contains the implementations of all repository traits defined in
//! `fts-core` for the SQLite database backend.

use crate::{
    Db,
    types::{BidderId, DateTime, DemandId, PortfolioId, ProductId},
};
use fts_core::ports::Repository;

mod batch;
mod demand;
mod portfolio;
mod product;

impl Repository for Db {
    type Error = sqlx::Error;
    type DateTime = DateTime;
    type BidderId = BidderId;
    type ProductId = ProductId;
    type DemandId = DemandId;
    type PortfolioId = PortfolioId;
}
