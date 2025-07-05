//! Type definitions for the SQLite implementation.
//!
//! This module contains both public types used throughout the crate and internal
//! types used for database row mapping. The public types include strongly-typed
//! IDs and datetime representations that ensure type safety across the system.

use fts_core::{
    models::{
        DemandCurve, DemandCurveDto, DemandGroup, DemandRecord, Map, PortfolioGroup,
        PortfolioRecord, ProductGroup, ValueRecord,
    },
    ports::Repository,
};

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
    pub id: DemandId,
    pub valid_from: DateTime,
    pub valid_until: Option<DateTime>,
    pub bidder_id: BidderId,
    pub app_data: sqlx::types::Json<AppData>,
    pub curve_data: Option<sqlx::types::Json<DemandCurveDto>>,
    pub portfolio_group: Option<sqlx::types::Json<PortfolioGroup<PortfolioId>>>,
}

impl<T, AppData> Into<DemandRecord<T, AppData>> for DemandRow<AppData>
where
    T: Repository<
            DateTime = DateTime,
            BidderId = BidderId,
            DemandId = DemandId,
            PortfolioId = PortfolioId,
            ProductId = ProductId,
        >,
{
    fn into(self) -> DemandRecord<T, AppData> {
        DemandRecord {
            id: self.id,
            valid_from: self.valid_from,
            valid_until: self.valid_until,
            bidder_id: self.bidder_id,
            app_data: self.app_data.0,
            curve_data: self
                .curve_data
                // SAFETY: we are deserialized from the database, and we ensure we only save valid demand curves
                .map(|x| unsafe { DemandCurve::new_unchecked(x.0) }),
            portfolio_group: self.portfolio_group.map(|x| x.0).unwrap_or_default(),
        }
    }
}

pub(crate) struct PortfolioRow<AppData> {
    pub id: PortfolioId,
    pub valid_from: DateTime,
    pub valid_until: Option<DateTime>,
    pub bidder_id: BidderId,
    pub app_data: sqlx::types::Json<AppData>,
    pub demand_group: Option<sqlx::types::Json<DemandGroup<DemandId>>>,
    pub product_group: Option<sqlx::types::Json<ProductGroup<ProductId>>>,
}

impl<T, AppData> Into<PortfolioRecord<T, AppData>> for PortfolioRow<AppData>
where
    T: Repository<
            DateTime = DateTime,
            BidderId = BidderId,
            DemandId = DemandId,
            PortfolioId = PortfolioId,
            ProductId = ProductId,
        >,
{
    fn into(self) -> PortfolioRecord<T, AppData> {
        PortfolioRecord {
            id: self.id,
            valid_from: self.valid_from,
            valid_until: self.valid_until,
            bidder_id: self.bidder_id,
            app_data: self.app_data.0,
            demand_group: self.demand_group.map(|x| x.0).unwrap_or_default(),
            product_group: self.product_group.map(|x| x.0).unwrap_or_default(),
        }
    }
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
