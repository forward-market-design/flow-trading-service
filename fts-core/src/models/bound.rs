use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A newtype wrapper around an optional float, with convenience methods to specify an infinite-fallback.
///
/// Bounds are used to represent constraint values that can either be finite numbers or
/// positive/negative infinity. When serialized, infinite values are represented as null
/// for cleaner API responses and requests.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct Bound(Option<f64>);

impl Bound {
    /// Returns the contained value or positive infinity if None
    ///
    /// This is typically used for upper bounds where no limit means infinity.
    pub fn or_pos_inf(&self) -> f64 {
        match self.0 {
            Some(x) => x,
            None => f64::INFINITY,
        }
    }

    /// Returns the contained value or negative infinity if None
    ///
    /// This is typically used for lower bounds where no limit means negative infinity.
    pub fn or_neg_inf(&self) -> f64 {
        match self.0 {
            Some(x) => x,
            None => f64::NEG_INFINITY,
        }
    }
}

impl From<f64> for Bound {
    /// Converts a float to a Bound, mapping infinite values to None
    fn from(value: f64) -> Self {
        if value.is_infinite() {
            Self(None)
        } else {
            Self(Some(value))
        }
    }
}
