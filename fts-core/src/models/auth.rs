use crate::models::{BidderId, Bound, ProductId, uuid_wrapper};
use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use thiserror::Error;
use time::OffsetDateTime;
use utoipa::ToSchema;

uuid_wrapper!(AuthId);

/// The supported constraints for an authorization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "RawAuthorization", into = "RawAuthorization")]
pub struct AuthData {
    /// The fastest rate at which the portfolio may sell (non-positive)
    pub min_rate: f64,
    /// The fastest rate at which the portfolio may buy (non-negative)
    pub max_rate: f64,
    /// A minimum amount of trade to preserve (always enforced against the authorization's contemporaneous amount of trade)
    pub min_trade: f64,
    /// A maximum amount of trade to preserve (always enforced against the authorization's contemporaneous amount of trade)
    pub max_trade: f64,
}

impl AuthData {
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
    #[error("NaN value encountered")]
    NAN,
    #[error("Rate restriction must allow for 0")]
    ZERORATE,
    #[error("Trade restriction is infeasible")]
    INFEASIBLETRADE,
}

/// The "DTO" type for AuthData. Omitted values default to the appropriately signed infinity.
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
#[derive(Serialize, Deserialize, PartialEq, ToSchema, Debug)]
pub struct AuthHistoryRecord {
    pub data: Option<AuthData>,
    #[serde(with = "time::serde::rfc3339")]
    pub version: OffsetDateTime,
}

/// A full description of an authorization
#[derive(Serialize, Deserialize, PartialEq, ToSchema, Debug)]
pub struct AuthRecord {
    /// The responsible bidder's id
    pub bidder_id: BidderId,

    /// A unique id for the auth
    pub auth_id: AuthId,

    /// The portfolio associated to the auth. Due to the expected size, this portfolio may be omitted from certain endpoints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<std::collections::HashMap<ProductId, f64>>)]
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

/// A portfolio is a weighted bundle of products
pub type Portfolio = IndexMap<ProductId, f64, FxBuildHasher>;

// Convert this record into the raw solver object, given a time-scale

impl AuthRecord {
    pub fn into_solver(self, scale: f64) -> Option<(AuthId, fts_solver::Auth<ProductId>)> {
        let trade = self.trade.unwrap_or_default();
        if let Some(data) = self.data {
            let min_trade = (data.min_trade - trade).max(data.min_rate * scale).min(0.0);
            let max_trade = (data.max_trade - trade).min(data.max_rate * scale).max(0.0);
            let portfolio = self
                .portfolio
                .unwrap_or_default()
                .into_iter()
                .collect::<fts_solver::Portfolio<_>>();

            if portfolio.len() == 0 {
                None
            } else {
                Some((
                    self.auth_id,
                    fts_solver::Auth {
                        min_trade,
                        max_trade,
                        portfolio,
                    },
                ))
            }
        } else {
            None
        }
    }
}
