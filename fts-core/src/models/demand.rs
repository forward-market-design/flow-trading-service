mod constant;
mod curve;

pub use constant::{Constant, RawConstant};
pub use curve::{Curve, Point};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

/// A demand curve over (rate, price).
///
/// DemandCurve represents either:
/// - A non-increasing, piecewise-linear demand curve assigning a cost to each rate in its domain, or
/// - A simple, "flat" demand curve assining a constant cost to each rate in its domain.
///
/// This is the core component that defines how a bidder values different trade outcomes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(untagged, try_from = "RawDemandCurve", into = "RawDemandCurve")]
// TODO: Utoipa doesn't fully support all the Serde annotations,
// so we injected `untagged` (which Serde will ignore given the presence of
// of `try_from` and `into`), then inline the actual fields. This appears
// to correctly generate the OpenAPI schema, but we should revisit.
pub enum DemandCurve {
    /// A piecewise linear demand curve defined by points
    Curve(#[schema(inline)] Curve),
    /// A constant constraint enforcing a specific trade quantity at a price
    Constant(#[schema(inline)] Constant),
}

impl DemandCurve {
    /// Return the domain of the demand curve (min and max rates)
    pub fn domain(&self) -> (f64, f64) {
        match self {
            Self::Constant(constant) => constant.domain(),
            Self::Curve(curve) => curve.domain(),
        }
    }

    /// Convert the curve data into a solver-specific representation
    pub fn as_solver(&self, scale: f64) -> Vec<fts_solver::Point> {
        match self {
            Self::Constant(constant) => constant.as_solver(scale),
            Self::Curve(curve) => curve.as_solver(scale),
        }
    }
}

/// An error type for the ways in which the provided utility function may be invalid.
#[derive(Error, Debug)]
pub enum ValidationError {
    /// Error when a curve's definition is invalid
    #[error("invalid demand curve: {0}")]
    Curve(#[from] curve::ValidationError),
    /// Error when a constant curve's definition is invalid
    #[error("invalid constant curve: {0}")]
    Constant(#[from] constant::ValidationError),
}

/// The "DTO" type for the utility
///
/// This enum represents the raw data formats accepted in API requests for defining costs.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum RawDemandCurve {
    /// A sequence of points defining a piecewise linear demand curve
    Curve(Vec<Point>),
    /// A raw constant constraint definition
    Constant(RawConstant),
}

impl TryFrom<RawDemandCurve> for DemandCurve {
    type Error = ValidationError;

    fn try_from(value: RawDemandCurve) -> Result<Self, Self::Error> {
        match value {
            RawDemandCurve::Curve(curve) => Ok(DemandCurve::Curve(curve.try_into()?)),
            RawDemandCurve::Constant(constant) => Ok(DemandCurve::Constant(constant.try_into()?)),
        }
    }
}

impl From<DemandCurve> for RawDemandCurve {
    fn from(value: DemandCurve) -> Self {
        match value {
            DemandCurve::Curve(curve) => RawDemandCurve::Curve(curve.into()),
            DemandCurve::Constant(constant) => RawDemandCurve::Constant(constant.into()),
        }
    }
}
