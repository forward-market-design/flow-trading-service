use std::cmp::Ordering;

/// A representation of a point for use in defining piecewise-linear curves
///
/// Each point consists of:
/// - A rate (quantity per time unit)
/// - A price (value per unit)
///
/// Points are used to define the vertices of piecewise-linear demand curves.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema), schemars(inline))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Point {
    /// The rate (quantity per time) coordinate
    pub rate: f64,
    /// The price (value per unit) coordinate
    pub price: f64,
}

// We define a partial ordering for point so that demand curve validation is:
// All consecutive pairs of points satisfy pt0 <= pt1
// This means: pt0.rate <= pt1.rate AND pt0.price >= pt1.price
impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let rate_ord = self.rate.partial_cmp(&other.rate)?;
        let price_ord = self.price.partial_cmp(&other.price)?.reverse();
        // Note the reversed state for price!

        if rate_ord == Ordering::Equal {
            Some(price_ord)
        } else if price_ord == Ordering::Equal {
            Some(rate_ord)
        } else if rate_ord == price_ord {
            Some(price_ord)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point {
        Point { rate: x, price: y }
    }

    #[test]
    fn test_good_cmp() {
        let a = pt(0.0, 10.0);
        let b = pt(10.0, 0.0);
        assert!(a < b);
        assert!(a <= b);
        assert!(b > a);
        assert!(b >= a);
        assert!(a == a);
    }

    #[test]
    fn test_bad_cmp() {
        let a = pt(0.0, 10.0);
        let b = pt(10.0, 20.0);
        assert!(a.partial_cmp(&b).is_none());
        assert!(b.partial_cmp(&a).is_none());
    }

    #[test]
    fn test_nan() {
        let a = pt(f64::NAN, 10.0);
        let b = pt(10.0, f64::NAN);
        assert!(a.partial_cmp(&b).is_none());
        assert!(b.partial_cmp(&a).is_none());
    }
}
