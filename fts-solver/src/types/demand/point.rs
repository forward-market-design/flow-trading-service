use std::cmp::Ordering;

/// A demand curve is defined by its points, which in turn have an associated `quantity` and `price`
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Point {
    /// The quantity associated to the point (typically the dependent variable)
    pub quantity: f64,
    /// The price associated to the point (typically the independent variable)
    pub price: f64,
}

// We define a partial ordering for point so that demand curve validation is:
// All consecutive pairs of points satisfy pt0.partial_cmp(&pt1).is_le()
impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (
            self.quantity.partial_cmp(&other.quantity),
            self.price.partial_cmp(&other.price),
        ) {
            (Some(Ordering::Less), Some(price)) => {
                if price.is_ge() {
                    Some(Ordering::Less)
                } else {
                    None
                }
            }
            (Some(Ordering::Greater), Some(price)) => {
                if price.is_le() {
                    Some(Ordering::Greater)
                } else {
                    None
                }
            }
            (Some(Ordering::Equal), Some(price)) => Some(price.reverse()),
            (None, _) | (_, None) => None,
        }
    }
}
