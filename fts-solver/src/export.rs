use crate::{HashSet, Submission};
use std::fmt::Display;
use std::hash::Hash;
use std::io::Write;

/// Convert a set of flow trading submissions to a quadratic program and export
/// this program to `.mps` format.
pub fn export_mps<
    T,
    BidderId: Display + Eq + Hash + Clone + Ord,
    PortfolioId: Display + Eq + Hash + Clone + Ord,
    ProductId: Display + Eq + Hash + Clone + Ord,
>(
    auction: &T,
    buffer: &mut impl Write,
) -> Result<(), std::io::Error>
where
    for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<PortfolioId, ProductId>)>,
{
    // MPS is a somewhat archaic format, but is easy enough to generate.
    // https://www.ibm.com/docs/en/icos/22.1.2?topic=standard-records-in-mps-format
    // is a good reference.

    writeln!(buffer, "NAME flow_trade_qp")?;
    writeln!(buffer, "ROWS")?;

    // Our objective is gains-from-trade ("gft")
    writeln!(buffer, " N    gft")?;

    // We will have one row per product, set equal to zero, which will give us the shadow prices
    let mut products = auction
        .into_iter()
        .flat_map(|(_, submission)| submission.portfolios.values())
        .flat_map(|portfolio| portfolio.keys())
        .collect::<HashSet<_>>();
    products.sort_unstable();
    for product_id in products.iter() {
        // The product dual variables are named `p_{product_id}`
        writeln!(buffer, " E    p_{product_id}")?;
    }

    // We will also have one row per demand curve, set equal to zero.
    for (bidder_id, submission) in auction.into_iter() {
        // The group dual variables are named `g_{bidder_id}_{offset}`
        for (offset, _) in submission.demand_curves.iter().enumerate() {
            writeln!(buffer, " E    g_{bidder_id}_{offset}")?;
        }
    }

    // We have two sets of variables: the per-product allocations `x`, and
    // the demand curve segment fills `y`, related such that Ax - Σy = 0,
    // where the rows of Σ are disjoint 1 vectors.
    writeln!(buffer, "COLUMNS")?;

    // We begin with this first set x. Notably, these variables *do not* appear in the objective.
    for (bidder_id, submission) in auction.into_iter() {
        for (portfolio_id, portfolio) in submission.portfolios.iter() {
            for (product_id, weight) in portfolio.iter() {
                writeln!(
                    buffer,
                    "    x_{bidder_id}_{portfolio_id}    p_{product_id}    {weight}",
                )?;
            }

            for (offset, (group, _)) in submission.demand_curves.iter().enumerate() {
                if let Some(weight) = group.get(portfolio_id) {
                    writeln!(
                        buffer,
                        "    x_{bidder_id}_{portfolio_id}    g_{bidder_id}_{offset}    {weight}",
                    )?;
                }
            }
        }
    }

    // Now the second set y.
    for (bidder_id, submission) in auction.into_iter() {
        for (offset, (_, curve)) in submission.demand_curves.iter().enumerate() {
            for (idx, segment) in curve.iter().enumerate() {
                // MPS defaults to minimization. Further, the quadratic terms are specified in a
                // well-supported extension, so we only do the linear terms here.
                let (_, b) = segment.slope_intercept();
                writeln!(
                    buffer,
                    "    y_{bidder_id}_{offset}_{idx}    gft    {term}    g_{bidder_id}_{offset}    -1",
                    term = -b
                )?;
            }
        }
    }

    // Now we specify the domains for each variable.
    writeln!(buffer, "BOUNDS")?;
    for (bidder_id, submission) in auction.into_iter() {
        for portfolio_id in submission.portfolios.keys() {
            // The allocation variables are unconstrained.
            writeln!(buffer, " FR BND x_{bidder_id}_{portfolio_id}")?;
        }
    }
    for (bidder_id, submission) in auction.into_iter() {
        for (offset, (_, curve)) in submission.demand_curves.iter().enumerate() {
            for (idx, segment) in curve.iter().enumerate() {
                // The segment variables are bounded above and below (unless infinite)
                if segment.q0.is_finite() {
                    writeln!(
                        buffer,
                        " LO BND    y_{bidder_id}_{offset}_{idx}    {min}",
                        min = segment.q0,
                    )?;
                } else {
                    writeln!(buffer, " MI BND    y_{bidder_id}_{offset}_{idx}",)?;
                }
                if segment.q1.is_finite() {
                    writeln!(
                        buffer,
                        " UP BND    y_{bidder_id}_{offset}_{idx}    {max}",
                        max = segment.q1,
                    )?;
                } else {
                    writeln!(buffer, " PL BND    y_{bidder_id}_{offset}_{idx}",)?;
                }
            }
        }
    }

    // Finally, we leverage the quadratic extension
    writeln!(buffer, "QUADOBJ")?;
    for (bidder_id, submission) in auction.into_iter() {
        for (offset, (_, curve)) in submission.demand_curves.iter().enumerate() {
            for (idx, segment) in curve.iter().enumerate() {
                let (m, _) = segment.slope_intercept();
                writeln!(
                    buffer,
                    "    y_{bidder_id}_{offset}_{idx}    y_{bidder_id}_{offset}_{idx}    {term}",
                    term = -m
                )?;
            }
        }
    }

    writeln!(buffer, "ENDATA")?;
    Ok(())
}

/// Convert a set of flow trading submissions to a quadratic program and export
/// this program to `.lp` format.
pub fn export_lp<
    T,
    BidderId: Display + Eq + Hash + Clone + Ord,
    PortfolioId: Display + Eq + Hash + Clone + Ord,
    ProductId: Display + Eq + Hash + Clone + Ord,
>(
    auction: &T,
    buffer: &mut impl Write,
) -> Result<(), std::io::Error>
where
    for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<PortfolioId, ProductId>)>,
{
    // Start with the objective section - maximize gains from trade
    writeln!(buffer, "Maximize")?;

    // Start the objective function (gft)
    write!(buffer, "  gft: ")?;

    // Flag to track if we've written any terms yet
    let mut first_term = true;
    let mut has_quadratic_terms = false;
    // Add linear terms from the y variables
    for (bidder_id, submission) in auction.into_iter() {
        for (offset, (_, curve)) in submission.demand_curves.iter().enumerate() {
            for (idx, segment) in curve.iter().enumerate() {
                let (m, b) = segment.slope_intercept();
                has_quadratic_terms = has_quadratic_terms || m != 0.0;
                if first_term {
                    write!(buffer, "{b} y_{bidder_id}_{offset}_{idx}")?;
                    first_term = false;
                } else {
                    write!(buffer, " + {b} y_{bidder_id}_{offset}_{idx}")?;
                }
            }
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

        for (bidder_id, submission) in auction.into_iter() {
            for (offset, (_, curve)) in submission.demand_curves.iter().enumerate() {
                for (idx, segment) in curve.iter().enumerate() {
                    let (m, _) = segment.slope_intercept();
                    if first_term {
                        write!(buffer, "{m} y_{bidder_id}_{offset}_{idx} ^ 2")?;
                        first_term = false;
                    } else {
                        write!(buffer, " + {m} y_{bidder_id}_{offset}_{idx} ^ 2",)?;
                    }
                }
            }
        }

        write!(buffer, " ] / 2")?;
    }

    // End the objective line
    writeln!(buffer)?;

    // Constraints section
    writeln!(buffer, "Subject To")?;

    // Product constraints (all products must sum to zero)
    let mut products = auction
        .into_iter()
        .flat_map(|(_, submission)| submission.portfolios.values())
        .flat_map(|portfolio| portfolio.keys())
        .collect::<HashSet<_>>();
    products.sort_unstable();

    for product_id in products {
        // Start the constraint
        write!(buffer, "  p_{product_id}: ")?;

        // Flag to track if we've written any terms
        let mut first_term = true;

        // Collect all terms related to this product
        for (bidder_id, submission) in auction.into_iter() {
            for (portfolio_id, portfolio) in submission.portfolios.iter() {
                if let Some(&weight) = portfolio.get(product_id) {
                    if first_term {
                        write!(buffer, "{weight} x_{bidder_id}_{portfolio_id}")?;
                        first_term = false;
                    } else {
                        write!(buffer, " + {weight} x_{bidder_id}_{portfolio_id}",)?;
                    }
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
    for (bidder_id, submission) in auction.into_iter() {
        for (offset, (group, curve)) in submission.demand_curves.iter().enumerate() {
            // Start the constraint
            write!(buffer, "  g_{bidder_id}_{offset}: ")?;

            // Flag to track if we've written any terms
            let mut first_term = true;

            // Add terms for the x variables
            for (portfolio_id, &weight) in group.iter() {
                if first_term {
                    write!(buffer, "{weight} x_{bidder_id}_{portfolio_id}")?;
                    first_term = false;
                } else {
                    write!(buffer, " + {weight} x_{bidder_id}_{portfolio_id}",)?;
                }
            }

            // Assertion: first_term = false, since groups are non empty

            // Add terms for the y variables (with negative coefficients)
            for (idx, _) in curve.iter().enumerate() {
                write!(buffer, " - y_{bidder_id}_{offset}_{idx}")?;
            }

            // Finish the constraint: = 0
            writeln!(buffer, " = 0")?;
        }
    }

    // Bounds section
    writeln!(buffer, "Bounds")?;

    // The x variables are unconstrained (free)
    for (bidder_id, submission) in auction.into_iter() {
        for portfolio_id in submission.portfolios.keys() {
            writeln!(buffer, "  x_{bidder_id}_{portfolio_id} free")?;
        }
    }

    // The y variables have specific bounds
    for (bidder_id, submission) in auction.into_iter() {
        for (offset, (_, curve)) in submission.demand_curves.iter().enumerate() {
            for (idx, segment) in curve.iter().enumerate() {
                match (segment.q0.is_finite(), segment.q1.is_finite()) {
                    (true, true) => {
                        writeln!(
                            buffer,
                            "  {min} <= y_{bidder_id}_{offset}_{idx} <= {max}",
                            min = segment.q0,
                            max = segment.q1
                        )?;
                    }
                    (true, false) => {
                        writeln!(
                            buffer,
                            "  y_{bidder_id}_{offset}_{idx} >= {min}",
                            min = segment.q0
                        )?;
                    }
                    (false, true) => {
                        writeln!(
                            buffer,
                            "  y_{bidder_id}_{offset}_{idx} <= {max}",
                            max = segment.q1
                        )?;
                    }
                    (false, false) => {
                        writeln!(buffer, "  y_{bidder_id}_{offset}_{idx} free")?;
                    }
                }
            }
        }
    }

    // End the LP file
    writeln!(buffer, "End")?;

    Ok(())
}
