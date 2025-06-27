use crate::models::Point;

/// A representation of a flat demand curve supporting interval, half-line, and full-line trading domains
///
/// A constant curve represents a fixed price for trades within a specified rate interval.
/// This can be used to express indifference to (potentially unbounded) trade rates at a specific price.
///
/// The sign convention follows flow trading standards:
/// - min_rate ≤ 0 (non-positive): maximum selling rate
/// - max_rate ≥ 0 (non-negative): maximum buying rate
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "ConstantCurveDto", into = "ConstantCurveDto")
)]
pub struct ConstantCurve {
    min_rate: f64,
    max_rate: f64,
    price: f64,
}

impl ConstantCurve {
    /// Creates a new constant constraint without validation
    ///
    /// # Safety
    /// This function is unsafe because it bypasses validation of rates and price.
    /// It should only be used when the caller can guarantee the values are valid.
    pub unsafe fn new_unchecked(min_rate: f64, max_rate: f64, price: f64) -> Self {
        Self {
            min_rate: min_rate,
            max_rate: max_rate,
            price: price,
        }
    }

    /// Creates a new constant curve with validation
    pub fn new(
        min_rate: Option<f64>,
        max_rate: Option<f64>,
        price: f64,
    ) -> Result<Self, ConstantCurveError> {
        let dto = ConstantCurveDto {
            min_rate,
            max_rate,
            price,
        };
        Self::try_from(dto)
    }

    /// Return the domain of the demand curve (min and max rates)
    pub fn domain(&self) -> (f64, f64) {
        (self.min_rate, self.max_rate)
    }

    /// Returns the curve as a vector of points
    ///
    /// For a constant curve, this returns one or two points:
    /// - If min_rate equals max_rate: returns a single point
    /// - Otherwise: returns two points at the min and max rates, both with the same price
    pub fn points(self) -> Vec<Point> {
        let mut response = Vec::with_capacity(2);
        response.push(Point {
            rate: self.min_rate,
            price: self.price,
        });
        if self.min_rate != self.max_rate {
            response.push(Point {
                rate: self.max_rate,
                price: self.price,
            });
        }
        response
    }
}

/// A DTO to ensure that we always validate when we deserialize from an untrusted source
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema), schemars(inline))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct ConstantCurveDto {
    /// The minimum rate (nonpositive), defaulting to negative infinity if None
    pub min_rate: Option<f64>,
    /// The maximum rate (nonnegative), defaulting to positive infinity if None
    pub max_rate: Option<f64>,
    /// The (finite) price
    pub price: f64,
}

impl Into<ConstantCurveDto> for ConstantCurve {
    fn into(self) -> ConstantCurveDto {
        ConstantCurveDto {
            min_rate: if self.min_rate.is_finite() {
                Some(self.min_rate)
            } else {
                None
            },
            max_rate: if self.max_rate.is_finite() {
                Some(self.max_rate)
            } else {
                None
            },
            price: self.price,
        }
    }
}

impl TryFrom<ConstantCurveDto> for ConstantCurve {
    type Error = ConstantCurveError;

    fn try_from(value: ConstantCurveDto) -> Result<Self, Self::Error> {
        let min_rate = value.min_rate.unwrap_or(f64::NEG_INFINITY);
        let max_rate = value.max_rate.unwrap_or(f64::INFINITY);
        let price = value.price;

        if min_rate.is_nan() || max_rate.is_nan() || price.is_nan() {
            return Err(ConstantCurveError::NaN);
        }
        if price.is_infinite() {
            return Err(ConstantCurveError::InfinitePrice);
        }
        if !(min_rate <= 0.0 && 0.0 <= max_rate) {
            return Err(ConstantCurveError::ZeroTrade);
        }

        Ok(Self {
            min_rate,
            max_rate,
            price,
        })
    }
}

/// Errors that can occur when creating or validating a ConstantCurve
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ConstantCurveError {
    /// Error when any coordinate value is NaN
    #[error("NaN value encountered")]
    NaN,
    /// Error when the curve's domain does not include rate=0
    #[error("Domain excludes rate=0")]
    ZeroTrade,
    /// Error when the price is infinite
    #[error("Price cannot be infinite")]
    InfinitePrice,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_contains_zero() {
        // Valid: spans zero
        let result = ConstantCurve::new(Some(-5.0), Some(5.0), 10.0);
        assert!(result.is_ok());

        // Valid: min_rate exactly 0
        let result = ConstantCurve::new(Some(0.0), Some(5.0), 10.0);
        assert!(result.is_ok());

        // Valid: max_rate exactly 0
        let result = ConstantCurve::new(Some(-5.0), Some(0.0), 10.0);
        assert!(result.is_ok());

        // Valid: both None (infinite domain)
        let result = ConstantCurve::new(None, None, 10.0);
        assert!(result.is_ok());

        // Valid: min_rate None, max_rate positive
        let result = ConstantCurve::new(None, Some(5.0), 10.0);
        assert!(result.is_ok());

        // Valid: min_rate negative, max_rate None
        let result = ConstantCurve::new(Some(-5.0), None, 10.0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_neg_infinity() {
        let (min, max) = ConstantCurve::new(None, Some(5.0), 10.0).unwrap().domain();
        assert!(min.is_infinite() && min < 0.0 && max == 5.0);
    }

    #[test]
    fn test_pos_infinity() {
        let (min, max) = ConstantCurve::new(Some(0.0), None, 10.0).unwrap().domain();
        assert!(min == 0.0 && max.is_infinite() && max > 0.0);
    }

    #[test]
    fn test_full_infinity() {
        let (min, max) = ConstantCurve::new(None, None, 10.0).unwrap().domain();
        assert!(min.is_infinite() && min < 0.0 && max.is_infinite() && max > 0.0);
    }

    #[test]
    fn test_infinite_price() {
        assert_eq!(
            ConstantCurve::new(None, None, f64::INFINITY).unwrap_err(),
            ConstantCurveError::InfinitePrice
        );
    }

    #[test]
    fn test_nans() {
        assert_eq!(
            ConstantCurve::new(Some(f64::NAN), None, 10.0).unwrap_err(),
            ConstantCurveError::NaN
        );
        assert_eq!(
            ConstantCurve::new(None, Some(f64::NAN), 10.0).unwrap_err(),
            ConstantCurveError::NaN
        );
        assert_eq!(
            ConstantCurve::new(None, None, f64::NAN).unwrap_err(),
            ConstantCurveError::NaN
        );
    }

    #[test]
    fn test_bad_domain_reversed() {
        assert_eq!(
            ConstantCurve::new(Some(1.0), Some(-1.0), 10.0).unwrap_err(),
            ConstantCurveError::ZeroTrade
        );
    }

    #[test]
    fn test_bad_domain_strict_positive() {
        assert_eq!(
            ConstantCurve::new(Some(1.0), Some(3.0), 10.0).unwrap_err(),
            ConstantCurveError::ZeroTrade
        );
    }

    #[test]
    fn test_bad_domain_strict_negative() {
        assert_eq!(
            ConstantCurve::new(Some(-3.0), Some(-1.0), 10.0).unwrap_err(),
            ConstantCurveError::ZeroTrade
        );
    }
}
