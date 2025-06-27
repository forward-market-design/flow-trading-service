use fts_core::models::Point;

/// A single line segment satisfying q0 ≤ 0 ≤ q1 and p1 ≤ p0
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
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
            rate: mut q0,
            price: p0,
        } = a;
        let Point {
            rate: mut q1,
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

        let (q0, p0) = if m.is_infinite() || self.q0 >= qmin {
            (self.q0, self.p0)
        } else {
            (qmin, m * qmin + b)
        };

        let (q1, p1) = if m.is_infinite() || self.q1 <= qmax {
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
            rate: -2.0,
            price: 10.0,
        };
        let b = Point {
            rate: -1.0,
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
            rate: 1.0,
            price: 10.0,
        };
        let b = Point {
            rate: 2.0,
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
            rate: -2.0,
            price: 10.0,
        };
        let b = Point {
            rate: 3.0,
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
            rate: -1.0,
            price: 10.0,
        };
        let b = Point {
            rate: -2.0,
            price: 0.0,
        };

        assert!(Segment::new(a, b).is_err());
    }

    #[test]
    fn bad_demand_constructor() {
        let a = Point {
            rate: 2.0,
            price: 10.0,
        };
        let b = Point {
            rate: 1.0,
            price: 0.0,
        };

        assert!(Segment::new(a, b).is_err());
    }

    #[test]
    fn bad_arbitrage_constructor() {
        let a = Point {
            rate: 3.0,
            price: 10.0,
        };
        let b = Point {
            rate: -2.0,
            price: 0.0,
        };

        assert!(Segment::new(a, b).is_err());
    }

    #[test]
    fn finite_slope() {
        let a = Point {
            rate: -1.0,
            price: 4.0,
        };
        let b = Point {
            rate: 1.0,
            price: 0.0,
        };

        let (m, b) = Segment::new(a, b)
            .expect("valid interval")
            .0
            .slope_intercept();

        assert_eq!(m, -2.0);
        assert_eq!(b, 2.0);
    }

    #[test]
    fn infinite_slope() {
        let a = Point {
            rate: -0.0,
            price: 4.0,
        };
        let b = Point {
            rate: 0.0,
            price: 0.0,
        };

        let (m, b) = Segment::new(a, b)
            .expect("valid interval")
            .0
            .slope_intercept();

        assert_eq!(m, f64::NEG_INFINITY);
        assert_eq!(b, 2.0);
    }

    #[test]
    fn zero_slope_neg() {
        let a = Point {
            rate: f64::NEG_INFINITY,
            price: 4.0,
        };
        let b = Point {
            rate: 1.0,
            price: 0.0,
        };

        let (m, b) = Segment::new(a, b)
            .expect("valid interval")
            .0
            .slope_intercept();

        assert_eq!(m, 0.0);
        assert_eq!(b, 2.0);
    }

    #[test]
    fn zero_slope_pos() {
        let a = Point {
            rate: -1.0,
            price: 4.0,
        };
        let b = Point {
            rate: f64::INFINITY,
            price: 0.0,
        };

        let (m, b) = Segment::new(a, b)
            .expect("valid interval")
            .0
            .slope_intercept();

        assert_eq!(m, 0.0);
        assert_eq!(b, 2.0);
    }

    // Some simple data for the clip() tests
    fn finite_data() -> Segment {
        let a = Point {
            rate: -1.0,
            price: 4.0,
        };
        let b = Point {
            rate: 1.0,
            price: 0.0,
        };

        Segment::new(a, b).unwrap().0
    }

    #[test]
    fn clip_0() {
        let Segment { q0, q1, p0, p1 } = finite_data().clip(-5.0, 5.0).unwrap();
        assert_eq!(q0, -1.0);
        assert_eq!(q1, 1.0);
        assert_eq!(p0, 4.0);
        assert_eq!(p1, 0.0);
    }

    #[test]
    fn demand_clip_1() {
        let Segment { q0, q1, p0, p1 } = finite_data().clip(-5.0, 1.0).unwrap();
        assert_eq!(q0, -1.0);
        assert_eq!(q1, 1.0);
        assert_eq!(p0, 4.0);
        assert_eq!(p1, 0.0);
    }

    #[test]
    fn demand_clip_2() {
        let Segment { q0, q1, p0, p1 } = finite_data().clip(-5.0, 0.5).unwrap();
        assert_eq!(q0, -1.0);
        assert_eq!(q1, 0.5);
        assert_eq!(p0, 4.0);
        assert_eq!(p1, 1.0);
    }

    #[test]
    fn demand_clip_3() {
        let Segment { q0, q1, p0, p1 } = finite_data().clip(-5.0, 0.0).unwrap();
        assert_eq!(q0, -1.0);
        assert_eq!(q1, 0.0);
        assert_eq!(p0, 4.0);
        assert_eq!(p1, 2.0);
    }

    #[test]
    fn supply_clip_1() {
        let Segment { q0, q1, p0, p1 } = finite_data().clip(-1.0, 5.0).unwrap();
        assert_eq!(q0, -1.0);
        assert_eq!(q1, 1.0);
        assert_eq!(p0, 4.0);
        assert_eq!(p1, 0.0);
    }

    #[test]
    fn supply_clip_2() {
        let Segment { q0, q1, p0, p1 } = finite_data().clip(-0.5, 5.0).unwrap();
        assert_eq!(q0, -0.5);
        assert_eq!(q1, 1.0);
        assert_eq!(p0, 3.0);
        assert_eq!(p1, 0.0);
    }

    #[test]
    fn supply_clip_3() {
        let Segment { q0, q1, p0, p1 } = finite_data().clip(0.0, 5.0).unwrap();
        assert_eq!(q0, 0.0);
        assert_eq!(q1, 1.0);
        assert_eq!(p0, 2.0);
        assert_eq!(p1, 0.0);
    }
}
