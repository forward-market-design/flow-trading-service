use super::spvec;
use std::cmp::Ordering;

// A group is a linear combination of portfolios
spvec!(Group);

/// A basic type for representing a point on a piecewise-linear demand curve
#[derive(Clone, PartialEq, Debug)]
pub struct Point {
    pub quantity: f64,
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

/// A piecewise-linear, weakly monotone curve for representing demand
#[derive(Debug)]
pub struct PiecewiseLinearCurve {
    pub points: Vec<Point>,
}

/// A "flat" curve for representing demand, which may have a finite, half-line, or full-line domain.
#[derive(Debug)]
pub struct Constant {
    pub quantity: (f64, f64),
    pub price: f64,
}

/// The currently-supported ways to define utility functions
#[derive(Debug)]
pub enum Cost {
    PiecewiseLinearCurve(PiecewiseLinearCurve),
    Constant(Constant),
}
