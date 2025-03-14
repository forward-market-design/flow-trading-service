use super::spvec;
use std::cmp::Ordering;

// A group is a linear combination of portfolios
spvec!(Group);

/// A basic type for representing a point on a piecewise-linear demand curve
#[derive(Clone, PartialEq, Debug)]
pub struct Point {
    /// The quantity value at this point on the curve
    pub quantity: f64,
    /// The price value at this point on the curve
    pub price: f64,
}

// A partial ordering for points in the context of a weakly monotone piecewise linear curve

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.quantity.partial_cmp(&other.quantity) {
            Some(Ordering::Less) => {
                if self.price >= other.price {
                    Some(Ordering::Less)
                } else {
                    None
                }
            }
            Some(Ordering::Equal) => other.price.partial_cmp(&self.price),
            Some(Ordering::Greater) => {
                if self.price <= other.price {
                    Some(Ordering::Greater)
                } else {
                    None
                }
            }
            None => None,
        }
    }
}

/// A piecewise-linear, weakly monotone curve for representing demand.
/// Points should be ordered by quantity, and the curve should be non-increasing
/// (for typical demand curves) or non-decreasing (for typical supply curves).
#[derive(Debug)]
pub struct PiecewiseLinearCurve {
    /// The sequence of points defining the curve segments
    pub points: Vec<Point>,
}

/// A "flat" curve for representing demand, which may have a finite, half-line, or full-line domain.
#[derive(Debug)]
pub struct Constant {
    /// The quantity range (min, max) for which this constant price applies
    pub quantity: (f64, f64),
    /// The constant price value
    pub price: f64,
}

/// The currently-supported ways to define utility functions
#[derive(Debug)]
pub enum Cost {
    /// A piecewise-linear curve defining price as a function of quantity
    PiecewiseLinearCurve(PiecewiseLinearCurve),
    /// A constant price over a fixed quantity range
    Constant(Constant),
}
