use crate::models::{DateTimeRangeQuery, DateTimeRangeResponse, DemandRecord, DemandCurve, ValueRecord};

/// Repository interface for demand curve submission and retrieval.
///
/// This trait encapsulates all the functionality related to demand curve submission and retrieval.
/// Demands represent bidders' pricing preferences and are the fundamental input to the
/// optimization solver.
///
/// This trait is parameterized by a generic data type, allowing an application
/// to colocate write-once data alongside the relevant record.
pub trait DemandRepository<DemandData>: super::Repository {
    /// Get the bidder id associated to the demand
    fn get_demand_bidder_id(
        &self,
        demand_id: Self::DemandId,
    ) -> impl Future<Output = Result<Option<Self::BidderId>, Self::Error>> + Send;

    /// Create a new demand with an optional initial curve.
    fn create_demand(
        &self,
        demand_id: Self::DemandId,
        bidder_id: Self::BidderId,
        app_data: DemandData,
        curve_data: Option<DemandCurve>,
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Update the curve data for an existing demand.
    ///
    /// Setting curve_data to None effectively deactivates the demand
    /// while preserving its history.
    ///
    /// # Returns
    ///
    /// - Ok(true) if successful
    /// - Ok(false) if no such demand exists
    /// - Err otherwise
    fn update_demand(
        &self,
        demand_id: Self::DemandId,
        curve_data: Option<DemandCurve>,
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;

    /// Retrieve a demand at a specific point in time.
    ///
    /// Retrieve the requested demand curve and associated portfolios, returning Option::None if it does not exist.
    fn get_demand(
        &self,
        demand_id: Self::DemandId,
        as_of: Self::DateTime,
    ) -> impl Future<
        Output = Result<
            Option<
                DemandRecord<
                    Self::DateTime,
                    Self::BidderId,
                    Self::DemandId,
                    Self::PortfolioId,
                    DemandData,
                >,
            >,
            Self::Error,
        >,
    > + Send;

    /// Query all the demand curves with non-null data associated to any of `bidder_ids`
    /// as-of the specified time.
    ///
    /// # Returns
    ///
    /// A vector of demand IDs that have active curves at the specified time.
    fn query_demand(
        &self,
        bidder_ids: &[Self::BidderId],
        as_of: Self::DateTime,
    ) -> impl Future<Output = Result<Vec<Self::DemandId>, Self::Error>> + Send;

    /// Retrieve the history of curve changes for a demand.
    ///
    /// # Returns
    ///
    /// A paginated response containing historical curve records, including
    /// when the curve was created, modified, or deleted (None).
    fn get_demand_history(
        &self,
        demand_id: Self::DemandId,
        query: DateTimeRangeQuery<Self::DateTime>,
        limit: usize,
    ) -> impl Future<
        Output = Result<
            DateTimeRangeResponse<ValueRecord<Self::DateTime, DemandCurve>, Self::DateTime>,
            Self::Error,
        >,
    > + Send;
}
