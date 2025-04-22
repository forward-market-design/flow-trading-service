use super::demand;
use crate::models::{BidderId, Bound, DemandCurve, Group, ProductId, map_wrapper, uuid_wrapper};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use thiserror::Error;
use time::OffsetDateTime;
use utoipa::ToSchema;

uuid_wrapper!(AuthId);

/// An authorization defines a portfolio and associates some data. This data
/// describes any trading constraints, as well as a default demand curve to
/// associate to the portfolio.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "RawAuthorization", into = "RawAuthorization")]
pub struct AuthData {
    /// The demand curve to associate to the portfolio
    pub demand: DemandCurve,

    /// A minimum amount of trade to preserve (always enforced against the authorization's contemporaneous amount of trade)
    #[schema(value_type = Option<f64>)]
    pub min_trade: f64,

    /// A maximum amount of trade to preserve (always enforced against the authorization's contemporaneous amount of trade)
    #[schema(value_type = Option<f64>)]
    pub max_trade: f64,
}

impl AuthData {
    /// Creates a new AuthData with the specified constraints.
    pub fn new(
        demand: DemandCurve,
        min_trade: f64,
        max_trade: f64,
    ) -> Result<Self, ValidationError> {
        if min_trade.is_nan() || max_trade.is_nan() {
            return Err(ValidationError::NAN);
        }
        if min_trade > max_trade {
            return Err(ValidationError::INFEASIBLETRADE);
        }

        Ok(Self {
            demand,
            min_trade,
            max_trade,
        })
    }
}

/// An enumeration of the ways authorization data may be invalid
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Error when any constraint value is NaN
    #[error("NaN value encountered")]
    NAN,
    /// Error when demand curve is invalid
    #[error("Invalid demand curve: {0:?}")]
    DEMAND(demand::ValidationError),
    /// Error when min_trade > max_trade
    #[error("Trade restriction is infeasible")]
    INFEASIBLETRADE,
}

/// The "DTO" type for AuthData. Omitted values default to the appropriately signed infinity.
///
/// This provides a user-friendly interface for specifying auth constraints, allowing
/// missing values to default to appropriate infinities.
#[derive(Serialize, Deserialize)]
pub struct RawAuthorization {
    pub demand: demand::RawDemandCurve,
    pub min_trade: Bound,
    pub max_trade: Bound,
}

impl TryFrom<RawAuthorization> for AuthData {
    type Error = ValidationError;

    fn try_from(value: RawAuthorization) -> Result<Self, Self::Error> {
        AuthData::new(
            value
                .demand
                .try_into()
                .map_err(|err| ValidationError::DEMAND(err))?,
            value.min_trade.or_neg_inf(),
            value.max_trade.or_pos_inf(),
        )
    }
}

impl From<AuthData> for RawAuthorization {
    fn from(value: AuthData) -> Self {
        Self {
            demand: value.demand.into(),
            min_trade: value.min_trade.into(),
            max_trade: value.max_trade.into(),
        }
    }
}

/// A record of the authorization's data at the time it was updated or defined
///
/// This provides historical versioning of auth constraints, allowing the system
/// to track changes to auth parameters over time.
#[derive(Serialize, Deserialize, PartialEq, ToSchema, Debug)]
pub struct AuthHistoryRecord {
    /// The authorization constraints, or None if the auth was deactivated
    pub data: Option<AuthData>,
    /// The timestamp when this version was created
    #[serde(with = "time::serde::rfc3339")]
    pub version: OffsetDateTime,
}

/// A full description of an authorization
///
/// An AuthRecord combines all the information needed to define an authorization:
/// - Who owns it (bidder_id)
/// - What it trades (portfolio)
/// - How it can be traded (data)
/// - The current accumulated trade (trade)
#[derive(Serialize, Deserialize, PartialEq, ToSchema, Debug)]
pub struct AuthRecord {
    /// The responsible bidder's id
    pub bidder_id: BidderId,

    /// A unique id for the auth
    pub auth_id: AuthId,

    /// The portfolio associated to the auth. Due to the expected size, this portfolio may be omitted from certain endpoints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portfolio: Option<Portfolio>,

    /// The constraint data for the authorization
    pub data: Option<AuthData>,

    /// The "last-modified-or-created" time as recorded by the system
    #[serde(with = "time::serde::rfc3339")]
    pub version: OffsetDateTime,

    /// The amount of cumulative trade associated to this authorization, as-of the request time
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade: Option<f64>,
}

map_wrapper!(Portfolio, ProductId, f64);

impl AuthRecord {
    /// Converts this auth record into a solver-compatible format.
    ///
    /// This method applies the time scale to rate-based constraints and computes
    /// the appropriate trade bounds for the auction, taking into account the
    /// current accumulated trade.
    pub fn into_solver(
        self,
        scale: f64,
    ) -> Option<(
        Portfolio,
        fts_solver::DemandCurve<AuthId, Group, Vec<fts_solver::Point>>,
    )> {
        let trade = self.trade.unwrap_or_default();
        if let Some(data) = self.data {
            let (min_rate, max_rate) = data.demand.domain();
            let min_trade = (data.min_trade - trade).max(min_rate * scale).min(0.0);
            let max_trade = (data.max_trade - trade).min(max_rate * scale).max(0.0);

            Some((
                self.portfolio.unwrap_or_default(),
                fts_solver::DemandCurve {
                    domain: (min_trade, max_trade),
                    group: std::iter::once((self.auth_id, 1.0)).collect(),
                    points: data.demand.as_solver(scale),
                },
            ))
        } else {
            None
        }
    }
}
