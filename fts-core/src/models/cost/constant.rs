use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

use crate::models::Bound;

/// A representation of a flat demand curve supporting interval, half-line, and full-line trading domains
///
/// A constant constraint represents a fixed price for trades within a specified rate range.
/// This can be used to:
/// - Enforce exact trade quantities at a specific price
/// - Create price floors or ceilings
/// - Express indifference to trade quantity within a range at a specific price
///
/// The sign convention follows flow trading standards:
/// - min_rate ≤ 0 (non-positive): maximum selling rate
/// - max_rate ≥ 0 (non-negative): maximum buying rate
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "RawConstant", into = "RawConstant")]
pub struct Constant {
    /// The fastest rate at which the portfolio may sell (non-positive)
    #[schema(value_type = Option<f64>)]
    pub min_rate: f64,

    /// The fastest rate at which the portfolio may buy (non-negative)
    #[schema(value_type = Option<f64>)]
    pub max_rate: f64,

    /// The fixed price at which trades within the rate range are valued
    #[schema(value_type = Option<f64>)]
    pub price: f64,
}

/// The "DTO" type for Constant, allowing for infinite values to be represented as nulls
///
/// This provides a user-friendly interface for specifying constant constraints, allowing
/// missing values to default to appropriate infinities.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RawConstant {
    /// The minimum rate bound, defaulting to negative infinity if null
    pub min_rate: Bound,

    /// The maximum rate bound, defaulting to positive infinity if null
    pub max_rate: Bound,

    /// The fixed price for trades within the rate bounds
    pub price: f64,
}

impl TryFrom<RawConstant> for Constant {
    type Error = ValidationError;

    /// Attempts to convert from the DTO format to the internal representation,
    /// applying validation rules and handling infinite bounds.
    fn try_from(value: RawConstant) -> Result<Self, Self::Error> {
        Constant::new(
            value.min_rate.or_neg_inf(),
            value.max_rate.or_pos_inf(),
            value.price,
        )
    }
}

impl From<Constant> for RawConstant {
    /// Converts from the internal representation to the DTO format,
    /// mapping infinite values to null bounds.
    fn from(value: Constant) -> Self {
        Self {
            min_rate: value.min_rate.into(),
            max_rate: value.max_rate.into(),
            price: value.price,
        }
    }
}

impl Constant {
    /// Creates a new validated constant constraint
    ///
    /// Creates a constant constraint with the specified rates and price,
    /// ensuring all values satisfy validation requirements:
    /// - min_rate must be non-positive
    /// - max_rate must be non-negative
    /// - price must be finite
    /// - Neither rate nor price can be NaN
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

    /// Creates a new constant constraint without validation
    ///
    /// # Safety
    /// This function is unsafe because it bypasses validation of rates and price.
    /// It should only be used when the caller can guarantee the values are valid.
    pub unsafe fn new_unchecked(min_rate: f64, max_rate: f64, price: f64) -> Self {
        Self {
            min_rate,
            max_rate,
            price,
        }
    }
}

/// The various ways in which a flat demand curve can be invalid
#[derive(Debug, Error)]
#[error("(min_rate = {min_rate:?}, max_rate = {max_rate:?}, price = {price:?})")]
pub struct ValidationError {
    /// Error related to the min_rate value, if any
    min_rate: Option<BoundValidationError>,

    /// Error related to the max_rate value, if any
    max_rate: Option<BoundValidationError>,

    /// Error related to the price value, if any
    price: Option<PriceError>,
}

/// Errors specific to bound validation (min_rate and max_rate)
#[derive(Debug, Error)]
pub enum BoundValidationError {
    /// The bound value is NaN, which is not allowed
    #[error("NaN")]
    NAN,

    /// The bound value has incorrect sign (min_rate must be non-positive, max_rate must be non-negative)
    #[error("Sign")]
    SIGN,
}

/// Errors specific to price validation
#[derive(Debug, Error)]
pub enum PriceError {
    /// The price value is NaN, which is not allowed
    #[error("NaN")]
    NAN,

    /// The price value is infinite, which is not allowed
    #[error("Infinity")]
    INFINITY,
}

/// Validates that a bound value meets requirements
///
/// # Arguments
/// * `x` - The bound value to validate
/// * `sgn` - The sign to check against (1.0 for max_rate, -1.0 for min_rate)
///
/// # Returns
/// None if the bound is valid, or a BoundValidationError if invalid
fn validate_bound(x: f64, sgn: f64) -> Option<BoundValidationError> {
    if x.is_nan() {
        Some(BoundValidationError::NAN)
    } else if sgn * x < 0.0 {
        Some(BoundValidationError::SIGN)
    } else {
        None
    }
}

/// Validates that a price value meets requirements
///
/// # Arguments
/// * `x` - The price value to validate
///
/// # Returns
/// None if the price is valid, or a PriceError if invalid
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
    /// Converts this constant constraint to the solver's representation
    ///
    /// Applies the given time scale to rate-based constraints to produce
    /// quantity-based constraints for the solver
    pub fn as_solver(&self, scale: f64) -> fts_solver::Constant {
        fts_solver::Constant {
            quantity: (self.min_rate * scale, self.max_rate * scale),
            price: self.price,
        }
    }
}
