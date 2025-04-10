use super::{Point, Segment};

/// If a demand curve is an aggregation of individual demand segments, then we
/// can disaggregate a demand curve into these segments. This is useful for
/// constructing the optimization program.
pub fn disaggregate<T: Iterator<Item = Point>>(
    points: T,
    domain: (f64, f64),
) -> Option<impl Iterator<Item = Result<Segment, Segment>>> {
    if domain.1 < domain.0 {
        return None;
    }

    let mut points = points.peekable();

    if let Some(point) = points.peek() {
        let anchor = Some(if point.quantity >= domain.0 {
            // Will never panic because points.peek().is_some()
            points.next().unwrap()
        } else {
            Point {
                quantity: domain.0,
                price: point.price,
            }
        });

        Some(
            Disaggregation {
                points,
                anchor,
                domain,
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
    points: T,
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
        if let Some(prev) = self.anchor.take() {
            // If so, contemplate the next point.
            if let Some(next) = self.points.next() {
                // If there is a point, try to generate a segment.
                self.anchor = Some(next.clone());
                Segment::new(prev, next)
                    .map(|(demand, translate)| {
                        demand.clip(self.domain.0 - translate, self.domain.1 - translate)
                    })
                    .map_err(|(demand, _)| demand)
                    .transpose()
            } else {
                // If there are no more points, we are done iterating.
                // However, we might need to extrapolate one additional point.
                let next = Point {
                    quantity: self.domain.1,
                    price: prev.price,
                };
                Segment::new(prev, next)
                    .map(|(demand, translate)| {
                        demand.clip(self.domain.0 - translate, self.domain.1 - translate)
                    })
                    .map_err(|(demand, _)| demand)
                    .transpose()
            }
        } else {
            None
        }
    }
}
