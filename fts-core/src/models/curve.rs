//! Demand curve implementations for flow trading.
//!
//! This module provides different curve types to express bidders' pricing preferences:
//! - [`PwlCurve`]: Piecewise linear curves for complex pricing strategies
//! - [`ConstantCurve`]: Fixed price curves for simple trading strategies

mod constant;
mod pwl;

pub use constant::*;
pub use pwl::*;

// `schemars` does not support serde's try_from/into (https://github.com/GREsau/schemars/issues/210).
// Thus, the "parse" path necessarily diverges a bit between serde and schemars, which is unfortunate.
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema), schemars(untagged))]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "DemandCurveDto", into = "DemandCurveDto")
)]
#[derive(Clone, Debug)]
/// A demand curve expressing a bidder's willingness to pay at different rates.
///
/// The solver uses these curves to find optimal allocations that maximize total welfare.
/// All curves must include rate=0 in their domain to allow for zero trade scenarios.
pub enum DemandCurve {
    /// Piecewise linear curve defined by a series of points
    Pwl(#[cfg_attr(feature = "schemars", schemars(with = "PwlCurveDto"))] PwlCurve),
    /// Constant price curve over a rate interval
    Constant(#[cfg_attr(feature = "schemars", schemars(with = "ConstantCurveDto"))] ConstantCurve),
}

/// DTO for demand curves to enable validation during deserialization
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(untagged))]
#[derive(Debug)]
pub enum DemandCurveDto {
    /// Piecewise linear curve DTO
    Pwl(PwlCurveDto),
    /// Constant price curve DTO
    Constant(ConstantCurveDto),
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for DemandCurveDto {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde_untagged::UntaggedEnumVisitor::new()
            .seq(|seq| seq.deserialize().map(DemandCurveDto::Pwl))
            .map(|map| map.deserialize().map(DemandCurveDto::Constant))
            .deserialize(deserializer)
    }
}

impl TryFrom<DemandCurveDto> for DemandCurve {
    type Error = DemandCurveError;

    /// Creates a demand curve from a DTO, validating all constraints
    fn try_from(value: DemandCurveDto) -> Result<Self, Self::Error> {
        match value {
            DemandCurveDto::Pwl(curve) => Ok(curve.try_into()?),
            DemandCurveDto::Constant(constant) => Ok(constant.try_into()?),
        }
    }
}

impl Into<DemandCurveDto> for DemandCurve {
    fn into(self) -> DemandCurveDto {
        match self {
            Self::Pwl(curve) => DemandCurveDto::Pwl(curve.into()),
            Self::Constant(constant) => DemandCurveDto::Constant(constant.into()),
        }
    }
}

impl From<PwlCurve> for DemandCurve {
    fn from(value: PwlCurve) -> Self {
        Self::Pwl(value)
    }
}

impl From<ConstantCurve> for DemandCurve {
    fn from(value: ConstantCurve) -> Self {
        Self::Constant(value)
    }
}

impl TryFrom<PwlCurveDto> for DemandCurve {
    type Error = PwlCurveError;
    fn try_from(value: PwlCurveDto) -> Result<Self, Self::Error> {
        Ok(Self::Pwl(value.try_into()?))
    }
}

impl TryFrom<ConstantCurveDto> for DemandCurve {
    type Error = ConstantCurveError;
    fn try_from(value: ConstantCurveDto) -> Result<Self, Self::Error> {
        Ok(Self::Constant(value.try_into()?))
    }
}

/// Errors that can occur when constructing demand curves
#[derive(Debug, thiserror::Error)]
pub enum DemandCurveError {
    /// Error from constructing a piecewise linear curve
    #[error("invalid pwl curve: {0}")]
    Pwl(#[from] PwlCurveError),
    /// Error from constructing a constant curve
    #[error("invalid constant curve: {0}")]
    Constant(#[from] ConstantCurveError),
}

impl DemandCurve {
    /// Creates a demand curve without validation
    ///
    /// # Safety
    /// The caller must ensure the data represents a valid curve.
    /// Invalid curves may cause undefined behavior in the solver.
    pub unsafe fn new_unchecked(value: DemandCurveDto) -> Self {
        unsafe {
            match value {
                DemandCurveDto::Pwl(curve) => PwlCurve::new_unchecked(curve.0).into(),
                DemandCurveDto::Constant(ConstantCurveDto {
                    min_rate,
                    max_rate,
                    price,
                }) => ConstantCurve::new_unchecked(
                    min_rate.unwrap_or(f64::NEG_INFINITY),
                    max_rate.unwrap_or(f64::INFINITY),
                    price,
                )
                .into(),
            }
        }
    }

    /// Returns the rate interval over which this curve is defined
    ///
    /// # Returns
    /// A tuple `(min_rate, max_rate)` defining the valid rate range for this curve.
    pub fn domain(&self) -> (f64, f64) {
        match self {
            DemandCurve::Pwl(curve) => curve.domain(),
            DemandCurve::Constant(curve) => curve.domain(),
        }
    }

    /// Converts the curve into a vector of points
    ///
    /// For PWL curves, returns all defining points. For constant curves,
    /// returns two points representing the endpoints of the constant price segment.
    pub fn points(self) -> Vec<Point> {
        match self {
            DemandCurve::Pwl(curve) => curve.points(),
            DemandCurve::Constant(curve) => curve.points(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_pwl() {
        let raw = r#"[
            {
                "rate": 0.0,
                "price": 10.0
            },
            {
                "rate": 1.0,
                "price": 5.0
            }
        ]"#;

        let test = serde_json::from_str::<DemandCurve>(&raw);
        assert!(test.is_ok());
    }

    #[test]
    fn test_deserialize_constant() {
        let raw = r#"{
            "min_rate": -1.0,
            "max_rate": 1.0,
            "price": 10.0
        }"#;

        let test = serde_json::from_str::<DemandCurve>(&raw);
        assert!(test.is_ok());
    }
}
