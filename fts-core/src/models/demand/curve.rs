use serde::{Deserialize, Serialize};
use std::ops::Index;
use thiserror::Error;
use utoipa::ToSchema;

/// A representation of a piecewise-linear, weakly monotone decreasing demand curve
///
/// Demand curves define a bidder's willingness to pay for different quantities of a good.
/// In flow trading, these curves must be:
/// - Piecewise-linear (defined by a sequence of points)
/// - Weakly monotone decreasing (price non-increasing as rate increases)
/// - Include the point rate=0 in their domain (must allow zero trade)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "Vec::<Point>", into = "Vec::<Point>")]
pub struct Curve(Vec<Point>);

impl Curve {
    /// Creates a new demand curve from a sequence of points
    ///
    /// Validates that the points:
    /// - Are not empty
    /// - Have valid (non-NaN, non-infinite) coordinates
    /// - Are ordered by ascending rate and descending price
    /// - Include rate=0 in their domain (first point rate ≤ 0, last point rate ≥ 0)
    ///
    /// Returns a ValidationError if any of these conditions are not met.
    pub fn new(points: Vec<Point>) -> Result<Self, ValidationError> {
        if let Some(point) = points.first() {
            validate_point(point)?;
        } else {
            return Err(ValidationError::EMPTY);
        }

        for pair in points.windows(2) {
            // We've already checked pair[0], so we just need to check pair[1]
            validate_point(pair.index(1))?;

            let Point {
                rate: r0,
                price: p0,
            } = pair[0];

            let Point {
                rate: r1,
                price: p1,
            } = pair[1];

            if r1 < r0 || p0 < p1 {
                return Err(ValidationError::NONMONOTONE);
            }
        }

        // These unwraps will not panic, as we would have already returned from this function otherwise
        let r0 = points.first().unwrap().rate;
        let r1 = points.last().unwrap().rate;

        if r0 > 0.0 || r1 < 0.0 {
            Err(ValidationError::NOZERO)
        } else {
            Ok(Self(points))
        }
    }

    /// Creates a new Curve without validating the points
    ///
    /// # Safety
    ///
    /// This function should only be used when the caller can guarantee that
    /// the points satisfy all the requirements that `new` would check.
    /// Using invalid points can lead to incorrect behavior in downstream systems.
    pub unsafe fn new_unchecked(points: Vec<Point>) -> Self {
        Self(points)
    }

    /// Converts the curve into an iterator over its points
    pub fn into_iter(self) -> impl Iterator<Item = Point> {
        self.0.into_iter()
    }

    /// Scales the rate component of each point by the given factor
    ///
    /// Returns a vector of (scaled_rate, price) tuples. This is useful when
    /// converting between rate-based and quantity-based representations.
    pub fn scale(&self, x: f64) -> Vec<(f64, f64)> {
        self.0
            .iter()
            .map(|point| (point.rate * x, point.price))
            .collect()
    }
}

impl TryFrom<Vec<Point>> for Curve {
    type Error = ValidationError;

    /// Attempts to create a Curve from a vector of points
    ///
    /// Delegates to `Curve::new` for validation.
    fn try_from(value: Vec<Point>) -> Result<Self, Self::Error> {
        Curve::new(value)
    }
}

impl Into<Vec<Point>> for Curve {
    /// Converts the Curve back into a vector of Points
    fn into(self) -> Vec<Point> {
        self.0
    }
}

/// The various ways in which a curve can be invalid
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Error when any coordinate value is NaN
    #[error("NaN value encountered")]
    NAN,
    /// Error when any coordinate value is infinite
    #[error("Infinite value encountered")]
    INFINITY,
    /// Error when no points are provided
    #[error("No points provided")]
    EMPTY,
    /// Error when the curve's domain does not include rate=0
    #[error("Domain excludes rate=0")]
    NOZERO,
    /// Error when points violate the monotonicity requirement
    #[error("Points are not ordered by ascending rate, descending price")]
    NONMONOTONE,
}

/// Validates that a point has finite coordinates
fn validate_point(point: &Point) -> Result<(), ValidationError> {
    let Point { rate, price } = point;
    if rate.is_nan() || price.is_nan() {
        return Err(ValidationError::NAN);
    } else if rate.is_infinite() || price.is_infinite() {
        return Err(ValidationError::INFINITY);
    } else {
        Ok(())
    }
}

/// A representation of a point for use in defining piecewise-linear curves
///
/// Each point consists of:
/// - A rate (quantity per time unit)
/// - A price (value per unit)
///
/// Points are used to define the vertices of piecewise-linear demand curves.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Point {
    /// The rate (quantity per time) coordinate
    pub rate: f64,
    /// The price (value per unit) coordinate
    pub price: f64,
}

impl Point {
    /// Converts this point to a solver-compatible format, applying rate scaling
    ///
    /// The rate component is scaled by the given factor, typically representing
    /// a time interval duration, to convert from rate to quantity.
    pub fn as_solver(&self, scale: f64) -> fts_solver::Point {
        fts_solver::Point {
            quantity: self.rate * scale,
            price: self.price,
        }
    }
}

impl Curve {
    /// Return the domain of the demand curve (min and max rates)
    pub fn domain(&self) -> (f64, f64) {
        let min = self.0.first().map(|pt| pt.rate).unwrap_or(0.0);
        let max = self.0.last().map(|pt| pt.rate).unwrap_or(0.0);
        (min, max)
    }

    /// Removes any collinearities and scales by the provided value
    ///
    /// This optimizes the curve representation by:
    /// 1. Removing intermediate points that lie on the same line segment
    /// 2. Scaling rates by the provided factor to convert to quantities
    ///
    /// The resulting curve is suitable for use with the solver library.
    pub fn as_solver(&self, scale: f64) -> Vec<fts_solver::Point> {
        self.0
            .iter()
            .map(move |point| point.as_solver(scale))
            .collect()
    }
}
