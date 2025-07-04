//! Type definitions for the SQLite implementation.
//!
//! This module contains both public types used throughout the crate and internal
//! types used for database row mapping. The public types include strongly-typed
//! IDs and datetime representations that ensure type safety across the system.

use fts_core::models::{DemandCurveDto, DemandGroup, Map, ProductGroup, ValueRecord};

mod datetime;
pub use datetime::DateTime;

mod ids;
pub use ids::{BidderId, DemandId, PortfolioId, ProductId};

pub(crate) struct BatchData {
    pub demands: Option<sqlx::types::Json<Map<DemandId, DemandCurveDto>>>,
    pub portfolios: Option<
        sqlx::types::Json<Map<PortfolioId, (DemandGroup<DemandId>, ProductGroup<ProductId>)>>,
    >,
}

pub(crate) struct DemandRow<AppData> {
    pub bidder_id: BidderId,
    pub app_data: sqlx::types::Json<AppData>,
    pub curve_data: Option<sqlx::types::Json<DemandCurveDto>>,
    pub portfolio_group: Option<sqlx::types::Json<Map<PortfolioId>>>,
}

pub(crate) struct PortfolioRow<AppData> {
    pub bidder_id: BidderId,
    pub app_data: sqlx::types::Json<AppData>,
    pub demand_group: Option<sqlx::types::Json<DemandGroup<DemandId>>>,
    pub product_group: Option<sqlx::types::Json<ProductGroup<ProductId>>>,
}

pub(crate) struct ValueRow<Value> {
    pub valid_from: DateTime,
    pub valid_until: Option<DateTime>,
    pub value: sqlx::types::Json<Value>,
}

impl<T> Into<ValueRecord<DateTime, T>> for ValueRow<T> {
    fn into(self) -> ValueRecord<DateTime, T> {
        ValueRecord {
            valid_from: self.valid_from,
            valid_until: self.valid_until,
            value: self.value.0,
        }
    }
}
