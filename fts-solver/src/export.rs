use crate::{HashSet, Segment, disaggregate};
use fts_core::models::{DemandCurve, DemandGroup, Map, ProductGroup};
use std::fmt::Display;
use std::hash::Hash;
use std::io::Write;

/// Convert a set of flow trading submissions to a quadratic program and export
/// this program to `.mps` format.
pub fn export_mps<
    DemandId: Display + Eq + Hash + Clone,
    PortfolioId: Display + Eq + Hash + Clone,
    ProductId: Display + Eq + Hash + Clone + Ord,
>(
    demand_curves: Map<DemandId, DemandCurve>,
    portfolios: Map<PortfolioId, (DemandGroup<DemandId>, ProductGroup<ProductId>)>,
    buffer: &mut impl Write,
) -> Result<(), std::io::Error> {
    // MPS is a somewhat archaic format, but is easy enough to generate.
    // https://www.ibm.com/docs/en/icos/22.1.2?topic=standard-records-in-mps-format
    // is a good reference.

    // This prepare method canonicalizes the input in an appropriate manner
    let (demand_curves, portfolios, _, products) = super::prepare(demand_curves, portfolios);

    writeln!(buffer, "NAME flow_trade_qp")?;
    writeln!(buffer, "ROWS")?;

    // Our objective is gains-from-trade ("gft")
    writeln!(buffer, " N    gft")?;

    // We will have one row per product, set equal to zero, which will give us the shadow prices
    for product_id in products.keys() {
        // The product dual variables are named `p_{product_id}`
        writeln!(buffer, " E    p_{product_id}")?;
    }

    // We will also have one row per demand curve, set equal to zero.
    for demand_id in demand_curves.keys() {
        writeln!(buffer, " E    d_{demand_id}")?;
    }

    // We have two sets of variables: the per-product allocations `x`, and
    // the demand curve segment fills `y`, related such that Ax - Σy = 0,
    // where the rows of Σ are disjoint 1 vectors.
    writeln!(buffer, "COLUMNS")?;

    let mut zeroed = HashSet::default();

    // We begin with this first set x. Notably, these variables *do not* appear in the objective.
    for (portfolio_id, (demand_weights, product_weights)) in portfolios.iter() {
        for (product_id, weight) in product_weights.iter() {
            writeln!(buffer, "    x_{portfolio_id}    p_{product_id}    {weight}",)?;
        }
        for (demand_id, weight) in demand_weights.iter() {
            writeln!(buffer, "    x_{portfolio_id}    d_{demand_id}    {weight}",)?;
        }
        if product_weights.len() == 0 || demand_weights.len() == 0 {
            zeroed.insert(portfolio_id);
        }
    }

    // Now the second set y.
    let mut all_segments = Map::<(DemandId, usize), Segment>::default();
    for (demand_id, demand_curve) in demand_curves.into_iter() {
        let (min, max) = demand_curve.domain();
        let points = demand_curve.points();

        let segments = disaggregate(points.into_iter(), min, max).expect("empty demand curve");
        for (idx, segment) in segments.enumerate() {
            // TODO: propagate the error upwards
            let segment = segment.unwrap();

            // MPS defaults to minimization. Further, the quadratic terms are specified in a
            // well-supported extension, so we only do the linear terms here.
            let (_, b) = segment.slope_intercept();
            writeln!(
                buffer,
                "    y_{demand_id}_{idx}    gft    {term}    d_{demand_id}    -1",
                term = -b
            )?;

            // keep track of the segment
            all_segments.insert((demand_id.clone(), idx), segment);
        }
    }

    // Now we specify the domains for each variable.
    writeln!(buffer, "BOUNDS")?;
    for portfolio_id in portfolios.keys() {
        if zeroed.contains(portfolio_id) {
            writeln!(buffer, " FX BND    x_{portfolio_id} 0")?;
        } else {
            // The allocation variables are unconstrained typically.
            writeln!(buffer, " FR BND    x_{portfolio_id}")?;
        }
    }

    for ((demand_id, idx), segment) in all_segments.iter() {
        let min = segment.q0;
        let max = segment.q1;

        // The segment variables are bounded above and below (unless infinite)
        if min.is_finite() {
            writeln!(buffer, " LO BND    y_{demand_id}_{idx}    {min}",)?;
        } else {
            writeln!(buffer, " MI BND    y_{demand_id}_{idx}",)?;
        }
        if max.is_finite() {
            writeln!(buffer, " UP BND    y_{demand_id}_{idx}    {max}",)?;
        } else {
            writeln!(buffer, " PL BND    y_{demand_id}_{idx}",)?;
        }
    }

    // Finally, we leverage the quadratic extension
    writeln!(buffer, "QUADOBJ")?;
    for ((demand_id, idx), segment) in all_segments {
        let (m, _) = segment.slope_intercept();
        writeln!(
            buffer,
            "    y_{demand_id}_{idx}    y_{demand_id}_{idx}    {term}",
            term = -m
        )?;
    }

    writeln!(buffer, "ENDATA")?;
    Ok(())
}

/// Convert a set of flow trading submissions to a quadratic program and export
/// this program to `.lp` format.
pub fn export_lp<
    DemandId: Display + Eq + Hash + Clone,
    PortfolioId: Display + Eq + Hash + Clone,
    ProductId: Display + Eq + Hash + Clone + Ord,
>(
    demand_curves: Map<DemandId, DemandCurve>,
    portfolios: Map<PortfolioId, (DemandGroup<DemandId>, ProductGroup<ProductId>)>,
    buffer: &mut impl Write,
) -> Result<(), std::io::Error> {
    // This prepare method canonicalizes the input in an appropriate manner
    let (demand_curves, portfolios, _, products) = super::prepare(demand_curves, portfolios);
    let mut all_segments = Map::<(DemandId, usize), Segment>::default();

    // Start with the objective section - maximize gains from trade
    writeln!(buffer, "Maximize")?;

    // Start the objective function (gft)
    write!(buffer, "  gft: ")?;

    // Flag to track if we've written any terms yet
    let mut first_term = true;
    let mut has_quadratic_terms = false;

    // Add linear terms from the y variables
    for (demand_id, demand_curve) in demand_curves.iter() {
        let (min, max) = demand_curve.domain();
        let points = demand_curve.clone().points();

        let segments = disaggregate(points.into_iter(), min, max).expect("empty demand curve");
        for (idx, segment) in segments.enumerate() {
            // TODO: propagate the error upwards
            let segment = segment.unwrap();

            let (m, b) = segment.slope_intercept();
            has_quadratic_terms = has_quadratic_terms || m != 0.0;
            if first_term {
                write!(buffer, "{b} y_{demand_id}_{idx}")?;
                first_term = false;
            } else {
                write!(buffer, " + {b} y_{demand_id}_{idx}")?;
            }

            // keep track of the segment
            all_segments.insert((demand_id.clone(), idx), segment);
        }
    }

    // If no linear terms were written, add a 0
    if first_term {
        write!(buffer, "0")?;
    }

    if has_quadratic_terms {
        write!(buffer, " + [ ")?;

        // Reset first_term flag for quadratic terms
        first_term = true;

        for ((demand_id, idx), segment) in all_segments.iter() {
            let (m, _) = segment.slope_intercept();
            if first_term {
                write!(buffer, "{m} y_{demand_id}_{idx} ^ 2")?;
                first_term = false;
            } else {
                write!(buffer, " + {m} y_{demand_id}_{idx} ^ 2",)?;
            }
        }

        write!(buffer, " ] / 2")?;
    }

    // End the objective line
    writeln!(buffer)?;

    // Constraints section
    writeln!(buffer, "Subject To")?;

    // Product constraints (all products must sum to zero)
    for product_id in products.keys() {
        // Start the constraint
        write!(buffer, "  p_{product_id}: ")?;

        // Flag to track if we've written any terms
        let mut first_term = true;

        // Collect all terms related to this product
        for (portfolio_id, (_, product_weights)) in portfolios.iter() {
            if let Some(&weight) = product_weights.get(product_id) {
                if first_term {
                    write!(buffer, "{weight} x_{portfolio_id}")?;
                    first_term = false;
                } else {
                    write!(buffer, " + {weight} x_{portfolio_id}",)?;
                }
            }
        }

        // If no terms were written, add a 0
        if first_term {
            write!(buffer, "0")?;
        }

        // Finish the constraint: = 0
        writeln!(buffer, " = 0")?;
    }

    // Demand curve constraints
    for demand_id in demand_curves.keys() {
        // Start the constraint
        write!(buffer, "  d_{demand_id}: ")?;

        // Flag to track if we've written any terms
        let mut first_term = true;

        for (portfolio_id, (demand_weights, _)) in portfolios.iter() {
            if let Some(&weight) = demand_weights.get(demand_id) {
                if first_term {
                    write!(buffer, "{weight} x_{portfolio_id}")?;
                    first_term = false;
                } else {
                    write!(buffer, " + {weight} x_{portfolio_id}",)?;
                }
            }
        }

        // Assertion: first_term = false, since groups are non empty

        let mut key = (demand_id.clone(), 0);
        while all_segments.contains_key(&key) {
            write!(buffer, " - y_{demand_id}_{idx}", idx = key.1)?;
            key.1 += 1;
        }

        // Finish the constraint: = 0
        writeln!(buffer, " = 0")?;
    }

    // Bounds section
    writeln!(buffer, "Bounds")?;

    // The x variables are (typically) unconstrained (free)
    for (portfolio_id, (a, b)) in portfolios.iter() {
        if a.len() == 0 || b.len() == 0 {
            writeln!(buffer, "  x_{portfolio_id} = 0")?;
        } else {
            writeln!(buffer, "  x_{portfolio_id} free")?;
        }
    }

    // The y variables have specific bounds
    for ((demand_id, idx), segment) in all_segments {
        match (segment.q0.is_finite(), segment.q1.is_finite()) {
            (true, true) => {
                writeln!(
                    buffer,
                    "  {min} <= y_{demand_id}_{idx} <= {max}",
                    min = segment.q0,
                    max = segment.q1
                )?;
            }
            (true, false) => {
                writeln!(buffer, "  y_{demand_id}_{idx} >= {min}", min = segment.q0)?;
            }
            (false, true) => {
                writeln!(buffer, "  y_{demand_id}_{idx} <= {max}", max = segment.q1)?;
            }
            (false, false) => {
                writeln!(buffer, "  y_{demand_id}_{idx} free")?;
            }
        }
    }

    // End the LP file
    writeln!(buffer, "End")?;

    Ok(())
}
