use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

use crate::models::Bound;

/// A representation of a flat demand curve supporting interval, half-line, and full-line trading domains
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "RawConstant", into = "RawConstant")]
pub struct Constant {
    #[schema(value_type = Option<f64>)]
    pub min_rate: f64,
    #[schema(value_type = Option<f64>)]
    pub max_rate: f64,
    #[schema(value_type = Option<f64>)]
    pub price: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RawConstant {
    pub min_rate: Bound,
    pub max_rate: Bound,
    pub price: f64,
}

impl TryFrom<RawConstant> for Constant {
    type Error = ValidationError;

    fn try_from(value: RawConstant) -> Result<Self, Self::Error> {
        Constant::new(
            value.min_rate.or_neg_inf(),
            value.max_rate.or_pos_inf(),
            value.price,
        )
    }
}

impl From<Constant> for RawConstant {
    fn from(value: Constant) -> Self {
        Self {
            min_rate: value.min_rate.into(),
            max_rate: value.max_rate.into(),
            price: value.price,
        }
    }
}

impl Constant {
    pub fn new(min_rate: f64, max_rate: f64, price: f64) -> Result<Self, ValidationError> {
        // Use the unchecked variant to provide the default values for the
        // None's, but then painstakingly verify the validity.
        let unvalidated = unsafe { Self::new_unchecked(min_rate, max_rate, price) };

        let lerr = validate_bound(unvalidated.min_rate, -1.0);
        let rerr = validate_bound(unvalidated.max_rate, 1.0);
        let perr = validate_price(unvalidated.price);

        match (lerr, rerr, perr) {
            (None, None, None) => Ok(unvalidated),
            (lerr, rerr, perr) => Err(ValidationError {
                min_rate: lerr,
                max_rate: rerr,
                price: perr,
            }),
        }
    }

    pub unsafe fn new_unchecked(min_rate: f64, max_rate: f64, price: f64) -> Self {
        Self {
            min_rate,
            max_rate,
            price,
        }
    }
}

#[derive(Debug, Error)]
#[error("(min_rate = {min_rate:?}, max_rate = {max_rate:?}, price = {price:?})")]
pub struct ValidationError {
    min_rate: Option<BoundValidationError>,
    max_rate: Option<BoundValidationError>,
    price: Option<PriceError>,
}

#[derive(Debug, Error)]
pub enum BoundValidationError {
    #[error("NaN")]
    NAN,
    #[error("Sign")]
    SIGN,
}

#[derive(Debug, Error)]
pub enum PriceError {
    #[error("NaN")]
    NAN,
    #[error("Infinity")]
    INFINITY,
}

fn validate_bound(x: f64, sgn: f64) -> Option<BoundValidationError> {
    if x.is_nan() {
        Some(BoundValidationError::NAN)
    } else if sgn * x < 0.0 {
        Some(BoundValidationError::SIGN)
    } else {
        None
    }
}

fn validate_price(x: f64) -> Option<PriceError> {
    if x.is_nan() {
        Some(PriceError::NAN)
    } else if x.is_infinite() {
        Some(PriceError::INFINITY)
    } else {
        None
    }
}

impl Constant {
    pub fn as_solver(&self, scale: f64) -> fts_solver::Constant {
        fts_solver::Constant {
            quantity: (self.min_rate * scale, self.max_rate * scale),
            price: self.price,
        }
    }
}
