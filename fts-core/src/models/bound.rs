use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A newtype wrapper around an optional float, with convenience methods to specified an infinite-fallback
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct Bound(Option<f64>);

impl Bound {
    pub fn or_pos_inf(&self) -> f64 {
        match self.0 {
            Some(x) => x,
            None => f64::INFINITY,
        }
    }
    pub fn or_neg_inf(&self) -> f64 {
        match self.0 {
            Some(x) => x,
            None => f64::NEG_INFINITY,
        }
    }
}

impl From<f64> for Bound {
    fn from(value: f64) -> Self {
        if value.is_infinite() {
            Self(None)
        } else {
            Self(Some(value))
        }
    }
}
