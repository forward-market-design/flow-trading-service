use super::spvec;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

// A group is a linear combination of portfolios
spvec!(Group);

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct PiecewiseLinearCurve {
    pub points: Vec<Point>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Constant {
    pub quantity: (f64, f64),
    pub price: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Cost {
    PiecewiseLinearCurve(PiecewiseLinearCurve),
    Constant(Constant),
}
