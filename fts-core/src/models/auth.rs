use crate::models::{BidderId, Bound, ProductId, map_wrapper, uuid_wrapper};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use thiserror::Error;
use time::OffsetDateTime;
use utoipa::ToSchema;

use super::Group;

uuid_wrapper!(AuthId);

/// The supported constraints for an authorization.
///
/// An AuthData defines the trading constraints for an authorization:
/// - Rate constraints limit how fast a portfolio can be traded (in units per time)
/// - Trade constraints limit the total accumulated trade amount over time
///
/// The rate constraints must allow the possibility of zero trade (min_rate ≤ 0 ≤ max_rate).
///
/// The trade constraints do not have this restriction, but instead, at time of
/// specification, they *should* allow for the currently traded amount of the auth.
/// If they do not, the trade constraint is implicitly expanded to include 0 at
/// each auction, which may not be desired.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "RawAuthorization", into = "RawAuthorization")]
pub struct AuthData {
    /// The fastest rate at which the portfolio may sell (non-positive)
    #[schema(value_type = Option<f64>)]
    pub min_rate: f64,

    /// The fastest rate at which the portfolio may buy (non-negative)
    #[schema(value_type = Option<f64>)]
    pub max_rate: f64,

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
        min_rate: f64,
        max_rate: f64,
        min_trade: f64,
        max_trade: f64,
    ) -> Result<Self, ValidationError> {
        if min_rate.is_nan() || max_rate.is_nan() {
            return Err(ValidationError::NAN);
        }
        if !(min_rate <= 0.0 && 0.0 <= max_rate) {
            return Err(ValidationError::ZERORATE);
        }

        if min_trade.is_nan() || max_trade.is_nan() {
            return Err(ValidationError::NAN);
        }
        if min_trade > max_trade {
            return Err(ValidationError::INFEASIBLETRADE);
        }

        Ok(Self {
            min_rate,
            max_rate,
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
    /// Error when rate constraints don't allow zero trade
    #[error("Rate restriction must allow for 0")]
    ZERORATE,
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
    pub min_rate: Bound,
    pub max_rate: Bound,
    pub min_trade: Bound,
    pub max_trade: Bound,
}

impl TryFrom<RawAuthorization> for AuthData {
    type Error = ValidationError;

    fn try_from(value: RawAuthorization) -> Result<Self, Self::Error> {
        AuthData::new(
            value.min_rate.or_neg_inf(),
            value.max_rate.or_pos_inf(),
            value.min_trade.or_neg_inf(),
            value.max_trade.or_pos_inf(),
        )
    }
}

impl From<AuthData> for RawAuthorization {
    fn from(value: AuthData) -> Self {
        Self {
            min_rate: value.min_rate.into(),
            max_rate: value.max_rate.into(),
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
        impl ExactSizeIterator<Item = (ProductId, f64)>,
        fts_solver::DemandCurve<
            AuthId,
            indexmap::map::IntoIter<AuthId, f64>,
            std::vec::IntoIter<fts_solver::Point>,
        >,
    )> {
        let trade = self.trade.unwrap_or_default();
        if let Some(data) = self.data {
            let min_trade = (data.min_trade - trade).max(data.min_rate * scale).min(0.0);
            let max_trade = (data.max_trade - trade).min(data.max_rate * scale).max(0.0);

            Some((
                self.portfolio.unwrap_or_default().into_iter(),
                fts_solver::DemandCurve {
                    domain: (min_trade, max_trade),
                    group: std::iter::once((self.auth_id, 1.0))
                        .collect::<Group>()
                        .into_iter(),
                    points: vec![fts_solver::Point {
                        quantity: 0.0,
                        price: 0.0,
                    }]
                    .into_iter(),
                },
            ))
        } else {
            None
        }
    }
}
