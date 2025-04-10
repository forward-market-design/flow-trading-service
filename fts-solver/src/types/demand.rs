mod point;
pub use point::Point;

mod segment;
pub use segment::Segment;

mod disaggregate;
pub use disaggregate::disaggregate;

/// A demand curve represents utility via a piecewise linear function
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DemandCurve<Idx, A: Iterator<Item = (Idx, f64)>, B: Iterator<Item = Point>> {
    /// Constrains the otherwise-infinite domain of the function to q ∈ 𝒟
    pub domain: (f64, f64),
    /// The sparse vector that combines in an inner product with the portfolio variables
    pub group: A,
    /// The points that define a piecewise-linear curve, extrapolated to q = ±∞ via the nearest price
    pub points: B,
}
