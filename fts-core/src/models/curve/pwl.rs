mod point;
pub use point::Point;

/// A representation of a piecewise-linear, weakly monotone decreasing demand curve
///
/// Demand curves define a bidder's willingness to pay for different quantities of a good.
/// In flow trading, these curves must be:
/// - Piecewise-linear (defined by a sequence of points)
/// - Weakly monotone decreasing (price non-increasing as rate increases)
/// - Include the point rate=0 in their domain (must allow zero trade)
///
/// Unlike a `ConstantCurve`, all values (rates and prices) must be finite.
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "PwlCurveDto", into = "PwlCurveDto")
)]
pub struct PwlCurve(Vec<Point>);

impl PwlCurve {
    /// Creates a new PwlCurve from a vector of points, validating all constraints
    pub fn new(points: Vec<Point>) -> Result<Self, PwlCurveError> {
        let dto = PwlCurveDto(points);
        Self::try_from(dto)
    }

    /// Creates a new PwlCurve without validating the points
    ///
    /// # Safety
    ///
    /// This function bypasses all validation checks. The caller must guarantee that
    /// the points satisfy all requirements validated by [`PwlCurve::try_from`].
    /// Using invalid points can lead to incorrect behavior in downstream systems,
    /// particularly in the solver which assumes valid monotone curves.
    pub unsafe fn new_unchecked(points: Vec<Point>) -> Self {
        Self(points)
    }

    /// Returns the domain of the demand curve (min and max rates)
    ///
    /// # Returns
    ///
    /// A tuple `(min_rate, max_rate)` where:
    /// - `min_rate` is the rate of the first point (leftmost)
    /// - `max_rate` is the rate of the last point (rightmost)
    ///
    /// # Panics
    ///
    /// Panics if the curve has no points (which should never happen for a valid curve).
    pub fn domain(&self) -> (f64, f64) {
        (self.0.first().unwrap().rate, self.0.last().unwrap().rate)
    }

    /// Converts the curve into its constituent points
    ///
    /// This consumes the curve and returns the underlying vector of points.
    /// Useful for serialization or when the raw point data is needed.
    pub fn points(self) -> Vec<Point> {
        self.0
    }
}

/// DTO to ensure that we always validate when we deserialize from an untrusted source
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[derive(Debug)]
pub struct PwlCurveDto(pub Vec<Point>);

impl Into<PwlCurveDto> for PwlCurve {
    fn into(self) -> PwlCurveDto {
        PwlCurveDto(self.0)
    }
}

impl TryFrom<PwlCurveDto> for PwlCurve {
    type Error = PwlCurveError;

    /// Attempts to create a PwlCurve from a DTO, validating all constraints
    ///
    /// # Validation
    ///
    /// This function validates that:
    /// 1. The vector is not empty
    /// 2. No coordinate values are NaN
    /// 3. Points are ordered by ascending rate and descending price (monotonicity)
    /// 4. The curve domain includes rate=0 (allows zero trade)
    ///
    /// # Errors
    ///
    /// Returns `PwlCurveError` if any validation fails.
    fn try_from(value: PwlCurveDto) -> Result<Self, Self::Error> {
        if value.0.is_empty() {
            return Err(PwlCurveError::Empty);
        }

        let mut prev = Point {
            rate: f64::NEG_INFINITY,
            price: f64::INFINITY,
        };

        let mut negzero = false;
        let mut poszero = false;

        for point in value.0.iter() {
            // Check for NaN values
            if point.rate.is_nan() || point.price.is_nan() {
                return Err(PwlCurveError::NaN);
            }
            if point.rate.is_infinite() || point.price.is_infinite() {
                return Err(PwlCurveError::Infinity);
            }

            // Check monotonicity against previous point
            // Note that we need the negation here, and cannot just use `point < prev` as condition:
            // the comparison might yield None if the points are not comparable, which is quickly the case,
            // especially in the non-monotone case.
            if !(point >= &prev) {
                return Err(PwlCurveError::NonMonotone);
            }

            // Track whether the domain includes rate=0
            negzero = negzero || point.rate <= 0.0;
            poszero = poszero || point.rate >= 0.0;

            prev.rate = point.rate;
            prev.price = point.price;
        }

        // Ensure the curve allows zero trade (domain includes 0)
        if negzero && poszero {
            Ok(Self(value.0))
        } else {
            Err(PwlCurveError::ZeroTrade)
        }
    }
}

/// Errors that can occur when creating or validating a PwlCurve
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum PwlCurveError {
    /// Error when any coordinate value is NaN
    #[error("NaN value encountered")]
    NaN,
    /// Error when no points are provided
    #[error("No points provided")]
    Empty,
    /// Error when points violate the monotonicity requirement
    #[error("Points are not ordered by ascending rate, descending price")]
    NonMonotone,
    /// Error when the curve's domain does not include rate=0
    #[error("Domain excludes rate=0")]
    ZeroTrade,
    /// Error when a point has infinite rate or price
    #[error("Rates and prices cannot be infinite")]
    Infinity,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_pwl_curve() {
        assert_eq!(PwlCurve::new(vec![]).unwrap_err(), PwlCurveError::Empty);
    }

    #[test]
    fn test_nan_values_in_points() {
        // NaN in rate
        assert_eq!(
            PwlCurve::new(vec![
                Point {
                    rate: f64::NAN,
                    price: 10.0,
                },
                Point {
                    rate: 5.0,
                    price: 5.0,
                },
            ])
            .unwrap_err(),
            PwlCurveError::NaN,
        );

        // NaN in price
        assert_eq!(
            PwlCurve::new(vec![
                Point {
                    rate: 0.0,
                    price: f64::NAN,
                },
                Point {
                    rate: 5.0,
                    price: 5.0,
                },
            ])
            .unwrap_err(),
            PwlCurveError::NaN
        );

        // NaN in both
        assert_eq!(
            PwlCurve::new(vec![Point {
                rate: f64::NAN,
                price: f64::NAN,
            }])
            .unwrap_err(),
            PwlCurveError::NaN
        );
    }

    #[test]
    fn test_non_monotone_rate() {
        // Rates not in ascending order
        assert_eq!(
            PwlCurve::new(vec![
                Point {
                    rate: 5.0,
                    price: 10.0,
                },
                Point {
                    rate: 0.0,
                    price: 8.0,
                }, // rate goes backwards
                Point {
                    rate: 10.0,
                    price: 5.0,
                },
            ])
            .unwrap_err(),
            PwlCurveError::NonMonotone
        );
    }

    #[test]
    fn test_non_monotone_prices() {
        // Prices increasing when they should be non-increasing
        assert_eq!(
            PwlCurve::new(vec![
                Point {
                    rate: 0.0,
                    price: 5.0,
                },
                Point {
                    rate: 5.0,
                    price: 6.0,
                }, // price increases
                Point {
                    rate: 10.0,
                    price: 2.0,
                },
            ])
            .unwrap_err(),
            PwlCurveError::NonMonotone
        );
    }

    #[test]
    fn test_combined_non_monotonicity() {
        // Valid monotonicity: rates ascending, prices descending
        assert_eq!(
            PwlCurve::new(vec![
                Point {
                    rate: 0.0,
                    price: 5.0,
                },
                Point {
                    rate: 5.0,
                    price: 8.0,
                },
                Point {
                    rate: 10.0,
                    price: 12.0,
                },
            ])
            .unwrap_err(),
            PwlCurveError::NonMonotone
        );
    }

    #[test]
    fn test_domain_excludes_zero() {
        // All positive rates (no zero trade allowed)
        assert_eq!(
            PwlCurve::new(vec![
                Point {
                    rate: 1.0,
                    price: 10.0,
                },
                Point {
                    rate: 5.0,
                    price: 8.0,
                },
                Point {
                    rate: 10.0,
                    price: 5.0,
                },
            ])
            .unwrap_err(),
            PwlCurveError::ZeroTrade
        );

        // All negative rates (no zero trade allowed)
        assert_eq!(
            PwlCurve::new(vec![
                Point {
                    rate: -10.0,
                    price: 10.0,
                },
                Point {
                    rate: -5.0,
                    price: 8.0,
                },
                Point {
                    rate: -1.0,
                    price: 5.0,
                },
            ])
            .unwrap_err(),
            PwlCurveError::ZeroTrade
        );
    }

    #[test]
    fn test_edge_case_single_point_at_zero() {
        // Single point at exactly zero should be valid
        assert!(
            PwlCurve::new(vec![Point {
                rate: 0.0,
                price: 10.0,
            }])
            .is_ok()
        );
    }

    #[test]
    fn test_edge_case_duplicate_rates() {
        // Same rate with same price should be ok (though unusual)
        let result = PwlCurve::new(vec![
            Point {
                rate: 0.0,
                price: 10.0,
            },
            Point {
                rate: 5.0,
                price: 8.0,
            },
            Point {
                rate: 5.0,
                price: 8.0,
            }, // duplicate
            Point {
                rate: 10.0,
                price: 5.0,
            },
        ]);
        assert!(result.is_ok());

        // Same rate with different price, i.e. a step down
        let result = PwlCurve::new(vec![
            Point {
                rate: 0.0,
                price: 10.0,
            },
            Point {
                rate: 5.0,
                price: 8.0,
            },
            Point {
                rate: 5.0,
                price: 7.0,
            }, // same rate, lower price
            Point {
                rate: 10.0,
                price: 5.0,
            },
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_infinite_values() {
        // Positive infinity in rate
        assert_eq!(
            PwlCurve::new(vec![
                Point {
                    rate: 0.0,
                    price: 10.0,
                },
                Point {
                    rate: f64::INFINITY,
                    price: 0.0,
                },
            ])
            .unwrap_err(),
            PwlCurveError::Infinity
        ); // Should be valid

        // Negative infinity in rate
        assert_eq!(
            PwlCurve::new(vec![
                Point {
                    rate: f64::NEG_INFINITY,
                    price: 10.0,
                },
                Point {
                    rate: 0.0,
                    price: 5.0,
                },
                Point {
                    rate: 10.0,
                    price: 0.0,
                },
            ])
            .unwrap_err(),
            PwlCurveError::Infinity
        );

        // Infinity in price
        assert_eq!(
            PwlCurve::new(vec![
                Point {
                    rate: 0.0,
                    price: f64::INFINITY,
                },
                Point {
                    rate: 10.0,
                    price: 0.0,
                },
            ])
            .unwrap_err(),
            PwlCurveError::Infinity
        );
    }

    #[test]
    fn test_precision_edge_cases() {
        // Very small but valid differences
        let result = PwlCurve::new(vec![
            Point {
                rate: 0.0,
                price: 10.0,
            },
            Point {
                rate: f64::EPSILON,
                price: 10.0 - f64::EPSILON,
            },
        ]);
        assert!(result.is_ok());

        // Test with very small rates around zero
        let result = PwlCurve::new(vec![
            Point {
                rate: -f64::EPSILON,
                price: 10.0,
            },
            Point {
                rate: 0.0,
                price: 10.0,
            },
            Point {
                rate: f64::EPSILON,
                price: 10.0,
            },
            Point {
                rate: 1.0,
                price: 0.0,
            },
        ]);
        assert!(result.is_ok());
    }
}
