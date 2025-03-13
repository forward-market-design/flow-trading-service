use serde::{Deserialize, Serialize};
use std::ops::Index;
use thiserror::Error;
use utoipa::ToSchema;

/// A representation of a piecewise-linear, weakly monotone decreasing demand curve
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "Vec::<Point>", into = "Vec::<Point>")]
pub struct Curve(Vec<Point>);

impl Curve {
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

    pub unsafe fn new_unchecked(points: Vec<Point>) -> Self {
        Self(points)
    }

    pub fn into_iter(self) -> impl Iterator<Item = Point> {
        self.0.into_iter()
    }

    pub fn scale(&self, x: f64) -> Vec<(f64, f64)> {
        self.0
            .iter()
            .map(|point| (point.rate * x, point.price))
            .collect()
    }
}

impl TryFrom<Vec<Point>> for Curve {
    type Error = ValidationError;

    fn try_from(value: Vec<Point>) -> Result<Self, Self::Error> {
        Curve::new(value)
    }
}

impl Into<Vec<Point>> for Curve {
    fn into(self) -> Vec<Point> {
        self.0
    }
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("NaN value encountered")]
    NAN,
    #[error("Infinite value encountered")]
    INFINITY,
    #[error("No points provided")]
    EMPTY,
    #[error("Domain excludes rate=0")]
    NOZERO,
    #[error("Points are not ordered by ascending rate, descending price")]
    NONMONOTONE,
}

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Point {
    pub rate: f64,
    pub price: f64,
}

impl Point {
    pub fn as_solver(&self, scale: f64) -> fts_solver::Point {
        fts_solver::Point {
            quantity: self.rate * scale,
            price: self.price,
        }
    }
}

impl Curve {
    /// Removes any collinearities and scales by the provided value
    pub fn as_solver(&self, scale: f64) -> fts_solver::PiecewiseLinearCurve {
        // Start from the first point
        let mut reduced = vec![self.0.first().unwrap().as_solver(scale)];

        for i in 1..(self.0.len() - 1) {
            let x0 = reduced.last().unwrap();
            let x1 = unsafe { self.0.get_unchecked(i) }.as_solver(scale);
            let x2 = unsafe { self.0.get_unchecked(i + 1) }.as_solver(scale);
            // Going from the last accepted point, compare against the current point and the one after that.
            // If collinear, the slopes are the same. (This includes vertical, horizontal, and degenerate cases)
            if (x2.quantity - x0.quantity) * (x1.price - x0.price)
                != (x1.quantity - x0.quantity) * (x2.price - x0.price)
            {
                reduced.push(x1);
            }
        }

        let lst = self.0.last().unwrap().as_solver(scale);
        if lst != *reduced.last().unwrap() {
            reduced.push(lst);
        }

        fts_solver::PiecewiseLinearCurve { points: reduced }
    }
}
