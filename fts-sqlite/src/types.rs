//! Type definitions for the SQLite implementation.
//!
//! This module contains both public types used throughout the crate and internal
//! types used for database row mapping. The public types include strongly-typed
//! IDs and datetime representations that ensure type safety across the system.

use fts_core::models::{DemandCurve, DemandCurveDto, Map, ValueRecord};

mod datetime;
pub use datetime::DateTime;

mod ids;
pub use ids::{BidderId, DemandId, PortfolioId, ProductId};

pub(crate) struct BatchData {
    pub demand_curves: Option<sqlx::types::Json<Map<DemandId, DemandCurveDto>>>,
    pub demand_groups: Option<sqlx::types::Json<Map<PortfolioId, Map<DemandId>>>>,
    pub product_groups: Option<sqlx::types::Json<Map<PortfolioId, Map<ProductId>>>>,
}

pub(crate) struct DemandRow<AppData> {
    pub bidder_id: BidderId,
    pub app_data: sqlx::types::Json<AppData>,
    pub curve_data: Option<sqlx::types::Json<DemandCurveDto>>,
    pub portfolio_group: Option<sqlx::types::Json<Map<PortfolioId>>>,
}

pub(crate) struct DemandHistoryRow {
    pub valid_from: DateTime,
    pub valid_until: Option<DateTime>,
    pub curve_data: sqlx::types::Json<DemandCurveDto>,
}

impl Into<ValueRecord<DateTime, DemandCurve>> for DemandHistoryRow {
    fn into(self) -> ValueRecord<DateTime, DemandCurve> {
        ValueRecord {
            valid_from: self.valid_from,
            valid_until: self.valid_until,
            value: unsafe { DemandCurve::new_unchecked(self.curve_data.0) },
            // SAFETY: this is only being called when deserializing a SQL query, and we ensure curves
            //         are valid going into the database.
        }
    }
}

pub(crate) struct PortfolioDemandHistoryRow {
    pub valid_from: DateTime,
    pub valid_until: Option<DateTime>,
    pub demand_group: sqlx::types::Json<Map<DemandId>>,
}

impl Into<ValueRecord<DateTime, Map<DemandId>>> for PortfolioDemandHistoryRow {
    fn into(self) -> ValueRecord<DateTime, Map<DemandId>> {
        ValueRecord {
            valid_from: self.valid_from,
            valid_until: self.valid_until,
            value: self.demand_group.0,
        }
    }
}

pub(crate) struct PortfolioProductHistoryRow {
    pub valid_from: DateTime,
    pub valid_until: Option<DateTime>,
    pub product_group: sqlx::types::Json<Map<ProductId>>,
}

impl Into<ValueRecord<DateTime, Map<ProductId>>> for PortfolioProductHistoryRow {
    fn into(self) -> ValueRecord<DateTime, Map<ProductId>> {
        ValueRecord {
            valid_from: self.valid_from,
            valid_until: self.valid_until,
            value: self.product_group.0,
        }
    }
}

pub(crate) struct PortfolioRow<AppData> {
    pub bidder_id: BidderId,
    pub app_data: sqlx::types::Json<AppData>,
    pub demand_group: Option<sqlx::types::Json<Map<DemandId>>>,
    pub product_group: Option<sqlx::types::Json<Map<ProductId>>>,
}

pub(crate) struct OutcomeRow<Outcome> {
    pub as_of: DateTime,
    pub outcome: sqlx::types::Json<Outcome>,
}
