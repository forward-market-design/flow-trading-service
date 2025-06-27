use super::Segment;
use fts_core::models::Point;
use std::iter::Peekable;

/// If a demand curve is an aggregation of individual demand segments, then we
/// can disaggregate a demand curve into these segments. This is useful for
/// constructing the optimization program.
pub fn disaggregate<T: Iterator<Item = Point>>(
    points: T,
    min: f64,
    max: f64,
) -> Option<impl Iterator<Item = Result<Segment, Segment>>> {
    if !(min <= 0.0 && 0.0 <= max) {
        return None;
    }

    let mut points = points.peekable();

    if let Some(point) = points.peek() {
        let anchor = if point.rate < min {
            points.next()
        } else {
            Some(Point {
                rate: min,
                price: point.price,
            })
        };

        Some(
            Disaggregation {
                points,
                anchor,
                domain: (min, max),
            }
            // We remove any demand segments which do not contribute, but we preserve
            // any invalid segments in order to surface the error to the caller.
            .filter(|result| match result {
                Ok(demand) => demand.q0 != demand.q1,
                Err(_) => true,
            }),
        )
    } else {
        None
    }
}

// An iterator that disaggregates a demand curve into its simple segments
#[derive(Debug)]
struct Disaggregation<T: Iterator<Item = Point>> {
    /// The raw, underlying iterator of points
    points: Peekable<T>,
    /// An anchoring point, representing the "left" point of a sliding window of points
    anchor: Option<Point>,
    // A clipping domain. Since we validate domain.0 <= domain.1 in the caller, and the constructor is private, we can rely on this invariant
    domain: (f64, f64),
}

impl<T: Iterator<Item = Point>> Iterator for Disaggregation<T> {
    // If an Err() is returned, the original demand curve was invalid
    type Item = Result<Segment, Segment>;

    // Iterate over the translated segments of a demand curve
    fn next(&mut self) -> Option<Self::Item> {
        // Are we anchored?
        while let Some(prev) = self.anchor.take() {
            // If so, contemplate the next point.
            if self.domain.1 <= prev.rate {
                // early exit condition
                return None;
            } else if let Some(mut next) = self.points.next() {
                // If there is a point, try to generate a segment.
                loop {
                    // We remove any interior, collinear points to simplify the curve
                    if let Some(extra) = self.points.peek() {
                        if is_collinear(&next, &prev, extra) {
                            // Safe, since self.points.peek().is_some()
                            next = self.points.next().unwrap();
                            continue;
                        } else {
                            break;
                        }
                    } else {
                        if self.domain.1 > next.rate {
                            let extra = Point {
                                rate: self.domain.1,
                                price: next.price,
                            };
                            if is_collinear(&next, &prev, &extra) {
                                next = extra;
                            }
                        }
                        break;
                    }
                }

                self.anchor = Some(next.clone());

                let segment = Segment::new(prev, next)
                    .map(|(demand, translate)| {
                        demand.clip(self.domain.0 - translate, self.domain.1 - translate)
                    })
                    .map_err(|(demand, _)| demand)
                    .transpose();
                if segment.is_some() {
                    return segment;
                } else {
                    continue;
                }
            } else {
                // If there are no more points, we are done iterating.
                // However, we might need to extrapolate one additional point.
                let next = Point {
                    rate: self.domain.1,
                    price: prev.price,
                };

                return Segment::new(prev, next)
                    .map(|(demand, translate)| {
                        demand.clip(self.domain.0 - translate, self.domain.1 - translate)
                    })
                    .map_err(|(demand, _)| demand)
                    .transpose();
            }
        }

        None
    }
}

/// Is this point collinear with the other two?
fn is_collinear(pt: &Point, lhs: &Point, rhs: &Point) -> bool {
    let &Point {
        rate: x0,
        price: y0,
    } = lhs;
    let &Point {
        rate: x1,
        price: y1,
    } = pt;
    let &Point {
        rate: x2,
        price: y2,
    } = rhs;

    (x2 - x0) * (y1 - y0) == (x1 - x0) * (y2 - y0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> impl Iterator<Item = Point> {
        vec![
            Point {
                rate: -2.0,
                price: 4.0,
            },
            Point {
                rate: -1.0,
                price: 3.0,
            },
            Point {
                rate: 1.0,
                price: 1.0,
            },
            Point {
                rate: 2.0,
                price: 0.0,
            },
        ]
        .into_iter()
    }

    #[test]
    fn collinear_reduction() {
        let segments = disaggregate(data(), -2.0, 2.0)
            .unwrap()
            .map(|res| res.unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            segments,
            vec![Segment {
                q0: -2.0,
                q1: 2.0,
                p0: 4.0,
                p1: 0.0,
            }]
        )
    }

    #[test]
    fn extrapolate_bad() {
        assert!(disaggregate(data(), -10.0, -5.0).is_none());
        assert!(disaggregate(data(), 5.0, 10.0).is_none());
    }

    #[test]
    fn extrapolate_demand() {
        let segments = disaggregate(data(), 0.0, 5.0)
            .unwrap()
            .map(|res| res.unwrap())
            .collect::<Vec<_>>();

        let answer = vec![
            Segment {
                q0: 0.0,
                q1: 2.0,
                p0: 2.0,
                p1: 0.0,
            },
            Segment {
                q0: 0.0,
                q1: 3.0,
                p0: 0.0,
                p1: 0.0,
            },
        ];

        assert_eq!(segments, answer);
    }

    #[test]
    fn extrapolate_supply() {
        let segments = disaggregate(data(), -5.0, 0.0)
            .unwrap()
            .map(|res| res.unwrap())
            .collect::<Vec<_>>();

        let answer = vec![
            Segment {
                q0: -3.0,
                q1: 0.0,
                p0: 4.0,
                p1: 4.0,
            },
            Segment {
                q0: -2.0,
                q1: 0.0,
                p0: 4.0,
                p1: 2.0,
            },
        ];

        assert_eq!(segments, answer);
    }

    #[test]
    fn extrapolate_arbitrage() {
        let segments = disaggregate(data(), -5.0, 5.0)
            .unwrap()
            .map(|res| res.unwrap())
            .collect::<Vec<_>>();

        let answer = vec![
            Segment {
                q0: -3.0,
                q1: 0.0,
                p0: 4.0,
                p1: 4.0,
            },
            Segment {
                q0: -2.0,
                q1: 2.0,
                p0: 4.0,
                p1: 0.0,
            },
            Segment {
                q0: 0.0,
                q1: 3.0,
                p0: 0.0,
                p1: 0.0,
            },
        ];

        assert_eq!(segments, answer);
    }

    #[test]
    fn extrapolate_simple() {
        let segments = disaggregate(
            std::iter::once(Point {
                rate: 0.0,
                price: 5.0,
            }),
            -5.0,
            5.0,
        )
        .unwrap()
        .map(|res| res.unwrap())
        .collect::<Vec<_>>();

        let answer = vec![Segment {
            q0: -5.0,
            q1: 5.0,
            p0: 5.0,
            p1: 5.0,
        }];

        assert_eq!(segments, answer);
    }

    #[test]
    fn check_slope() {
        let points = vec![
            Point {
                rate: -2.0,
                price: 4.0,
            },
            Point {
                rate: 2.0,
                price: 5.0,
            },
        ];
        let segments = disaggregate(points.into_iter(), -1.0, 1.0)
            .unwrap()
            .collect::<Result<Vec<_>, _>>();
        assert!(segments.is_err());
    }
}
