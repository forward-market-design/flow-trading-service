use super::Point;

/// A single line segment satisfying q0 ≤ 0 ≤ q1 and p1 ≤ p0
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Segment {
    /// The supply associated to this segment (q0 ≤ 0)
    pub q0: f64,
    /// The demand associated to this segment (q1 ≥ 0)
    pub q1: f64,
    /// The bidding price for the supply
    pub p0: f64,
    /// The asking price for the demand
    pub p1: f64,
}

impl Segment {
    /// Construct a simple demand segment from two neighboring points on a demand curve.
    ///
    /// Does not check if the points are properly ordered.
    /// Additionally returns the amount the points were translated.
    pub unsafe fn new_unchecked(a: Point, b: Point) -> (Self, f64) {
        let Point {
            quantity: mut q0,
            price: p0,
        } = a;
        let Point {
            quantity: mut q1,
            price: p1,
        } = b;

        // This is subtle, but if you consider the possibilities:
        // * If q0 > 0, translate == q0
        // * If q1 < 0, translate == q1
        // * If q0 < 0 and q1 > 0, translate == 0
        // Accordingly, this will minimally translate the segment in order for it to contain q=0.
        let translate = q0.max(0.0) + q1.min(0.0);
        q0 -= translate;
        q1 -= translate;

        (Self { q0, q1, p0, p1 }, translate)
    }

    /// Construct a simple demand segment from two neighboring points on a demand curve,
    /// performing validation to ensure the result is valid.
    ///
    /// Additionally returns the amount the points were translated.
    pub fn new(a: Point, b: Point) -> Result<(Self, f64), (Self, f64)> {
        let ok = a <= b;
        let result = unsafe { Segment::new_unchecked(a, b) };
        if ok { Ok(result) } else { Err(result) }
    }

    /// Compute the slope and p-intercept of the line segment.
    pub fn slope_intercept(&self) -> (f64, f64) {
        let qmid = (self.q0 + self.q1) / 2.0;
        let pmid = (self.p0 + self.p1) / 2.0;
        if self.q0 == self.q1 {
            (f64::NEG_INFINITY, pmid)
        } else {
            let m = (self.p1 - self.p0) / (self.q1 - self.q0);
            let b = if qmid.is_finite() {
                pmid - m * qmid
            } else {
                pmid
            };
            (m, b)
        }
    }

    /// Clip the segment to the provided interval.
    ///
    /// Does not validate the requested interval.
    pub unsafe fn clip_unchecked(self, qmin: f64, qmax: f64) -> Self {
        let (m, b) = self.slope_intercept();

        let (q0, p0) = if self.q0 >= qmin {
            (self.q0, self.p0)
        } else {
            (qmin, m * qmin + b)
        };

        let (q1, p1) = if self.q1 <= qmax {
            (self.q1, self.p1)
        } else {
            (qmax, m * qmax + b)
        };

        Self { q0, q1, p0, p1 }
    }

    /// Clip the segment to the provided interval, returning a value if the interval is valid.
    pub fn clip(self, qmin: f64, qmax: f64) -> Option<Self> {
        if qmin <= 0.0 && qmax >= 0.0 {
            Some(unsafe { self.clip_unchecked(qmin, qmax) })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supply_constructor() {
        let a = Point {
            quantity: -2.0,
            price: 10.0,
        };
        let b = Point {
            quantity: -1.0,
            price: 0.0,
        };
        let (Segment { q0, q1, p0, p1 }, t) = Segment::new(a, b).expect("valid interval");

        assert_eq!(t, -1.0);
        assert_eq!(q0, -1.0);
        assert_eq!(q1, 0.0);
        assert_eq!(p0, 10.0);
        assert_eq!(p1, 0.0);
    }

    #[test]
    fn demand_constructor() {
        let a = Point {
            quantity: 1.0,
            price: 10.0,
        };
        let b = Point {
            quantity: 2.0,
            price: 0.0,
        };
        let (Segment { q0, q1, p0, p1 }, t) = Segment::new(a, b).expect("valid interval");

        assert_eq!(t, 1.0);
        assert_eq!(q0, 0.0);
        assert_eq!(q1, 1.0);
        assert_eq!(p0, 10.0);
        assert_eq!(p1, 0.0);
    }

    #[test]
    fn arbitrage_constructor() {
        let a = Point {
            quantity: -2.0,
            price: 10.0,
        };
        let b = Point {
            quantity: 3.0,
            price: 0.0,
        };
        let (Segment { q0, q1, p0, p1 }, t) = Segment::new(a, b).expect("valid interval");

        assert_eq!(t, 0.0);
        assert_eq!(q0, -2.0);
        assert_eq!(q1, 3.0);
        assert_eq!(p0, 10.0);
        assert_eq!(p1, 0.0);
    }

    #[test]
    fn bad_supply_constructor() {
        let a = Point {
            quantity: -1.0,
            price: 10.0,
        };
        let b = Point {
            quantity: -2.0,
            price: 0.0,
        };

        assert!(Segment::new(a, b).is_err());
    }

    #[test]
    fn bad_demand_constructor() {
        let a = Point {
            quantity: 2.0,
            price: 10.0,
        };
        let b = Point {
            quantity: 1.0,
            price: 0.0,
        };

        assert!(Segment::new(a, b).is_err());
    }

    #[test]
    fn bad_arbitrage_constructor() {
        let a = Point {
            quantity: 3.0,
            price: 10.0,
        };
        let b = Point {
            quantity: -2.0,
            price: 0.0,
        };

        assert!(Segment::new(a, b).is_err());
    }

    // TODO: test slope_intercept() and clip()
}
