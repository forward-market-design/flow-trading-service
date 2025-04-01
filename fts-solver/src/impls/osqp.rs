use crate::Map;
use crate::{
    AuctionOutcome, Auth, AuthOutcome, Constant, Point, ProductOutcome, Solver, Submission,
};
use core::f64;
use osqp::{CscMatrix, Problem, Settings, Status};
use std::hash::Hash;

/// A solver implementation that uses the OSQP (Operator Splitting Quadratic Program)
/// solver to find market clearing prices and trades.
///
/// OSQP uses the Alternating Direction Method of Multipliers (ADMM) approach,
/// which can be faster than interior point methods for large-scale problems,
/// though sometimes with lower precision.
pub struct OsqpSolver(Settings);

impl Default for OsqpSolver {
    fn default() -> Self {
        Self(Settings::default().verbose(false).polish(true))
    }
}

impl Solver for OsqpSolver {
    type Settings = Settings;

    fn new(settings: Self::Settings) -> Self {
        Self(settings)
    }

    fn solve<
        T,
        BidderId: Eq + Hash + Clone + Ord,
        AuthId: Eq + Hash + Clone + Ord,
        ProductId: Eq + Hash + Clone + Ord,
    >(
        &self,
        auction: &T,
        // TODO: warm-starts with the prices
    ) -> AuctionOutcome<BidderId, AuthId, ProductId>
    where
        for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<AuthId, ProductId>)>,
    {
        let (auths, products, ncosts) = super::prepare(auction);

        if products.len() == 0 {
            return AuctionOutcome::default();
        }

        // The trade and bid constraints are all (something) = 0, we need to
        // know how many of these there are in order to handle the box
        // constraints for each decision variable
        let nzero = products.len() + ncosts;

        // Our quadratic term is diagonal, so we build the matrix by defining its diagonal
        let mut p = Vec::new();
        // and these are the linear terms
        let mut q = Vec::new();

        // OSQP handles constraints via a box specification, e.g. lb <= Ax <= ub,
        // where equality is handled via setting lb[i] = ub[i].
        // The first `nzero` of lb and ub are =0 so we do that work upfront.
        let mut lb = vec![0.0; nzero];
        let mut ub = vec![0.0; nzero];

        // OSQP's matrix input is in the form of CSC, so we handle the memory representation
        // carefully.
        let mut a_nzval = Vec::new();
        let mut a_rowval = Vec::new();
        let mut a_colptr = Vec::new();

        // These will help us figure out the correct row index as we iterate.
        let mut group_offset = products.len();

        // We begin by setting up the portfolio variables
        for (_, submission) in auction.into_iter() {
            for (
                auth_id,
                Auth {
                    min_trade,
                    max_trade,
                    portfolio,
                },
            ) in submission.auths.iter()
            {
                // portfolio variables contribute nothing to the objective
                p.push(0.0);
                q.push(0.0);

                // start a new column in the constraint matrix
                a_colptr.push(a_nzval.len());

                // We copy the portfolio definition into the matrix
                for (product, &weight) in portfolio.iter() {
                    a_nzval.push(weight);
                    a_rowval.push(*products.get(product).unwrap());
                }

                // Now we embed the weights from each group. This loop is a little wonky;
                // in matrix terms, the representation is transposed in the wrong way.
                // However, we expect the number of groups to be fairly small, so simply
                // searching every group for every portfolio in the submission is not so bad.
                for group in submission
                    .cost_curves
                    .iter()
                    .map(|(group, _)| group)
                    .chain(submission.cost_constants.iter().map(|(group, _)| group))
                {
                    if let Some(x) = group.get(auth_id) {
                        a_nzval.push(x);
                        a_rowval.push(group_offset);
                        group_offset += 1;
                    }
                }

                // Now we add the box constraints.
                if min_trade.is_finite() || max_trade.is_finite() {
                    a_nzval.push(1.0);
                    a_rowval.push(lb.len());
                    lb.push(*min_trade);
                    ub.push(*max_trade);
                }
            }
        }

        // Now we setup the segment variables
        group_offset = products.len();
        for (_, submission) in auction.into_iter() {
            for (_, curve) in submission.cost_curves.iter() {
                for pair in curve.points.windows(2) {
                    // Extract the coordinates
                    let Point {
                        quantity: x0,
                        price: y0,
                    } = pair[0];
                    let Point {
                        quantity: x1,
                        price: y1,
                    } = pair[1];

                    // Slide the segment so that it abuts x=0
                    let (x0, x1) = {
                        let translate = x0.max(0.0) + x1.min(0.0);
                        (x0 - translate, x1 - translate)
                    };

                    let dx = x1 - x0;
                    let dy = y1 - y0;

                    // We ignore vertical segments
                    if dx == 0.0 {
                        continue;
                    }

                    // Setup the contributions to the objective
                    let quad = -dy / dx;
                    p.push(quad);
                    q.push(-(y0 + quad * x0));

                    // Insert a new column
                    a_colptr.push(a_nzval.len());

                    // Ensure it counts towards the group
                    a_nzval.push(-1.0);
                    a_rowval.push(group_offset);

                    // Setup the box constraints
                    a_nzval.push(1.0);
                    a_rowval.push(lb.len());
                    lb.push(x0);
                    ub.push(x1);
                }

                // Advance the group offset for the next bid/constraint
                group_offset += 1;
            }

            for (
                _,
                Constant {
                    quantity: (min, max),
                    price,
                },
            ) in submission.cost_constants.iter()
            {
                // Constraint segments contribute nothing to the objective, but otherwise
                // act a lot like a curve.
                p.push(0.0);
                q.push(-price);

                // Insert a new column
                a_colptr.push(a_nzval.len());

                // Ensure it counts towards the group
                a_nzval.push(-1.0);
                a_rowval.push(group_offset);

                // Also establish the relevant constraints
                if min.is_finite() || max.is_finite() {
                    a_nzval.push(1.0);
                    a_rowval.push(lb.len());
                    lb.push(*min);
                    ub.push(*max);
                }

                // Advance the group offset for the next bid/constraint
                group_offset += 1;
            }
        }

        // We need to polish off the CSC matrix
        a_colptr.push(a_nzval.len());

        let m = lb.len();
        let n = p.len();

        let a_matrix = CscMatrix {
            nrows: m,
            ncols: n,
            indptr: a_colptr.into(),
            indices: a_rowval.into(),
            data: a_nzval.into(),
        };

        // Finally, we need to convert our p spec into a csc matrix
        let p_matrix = {
            CscMatrix {
                nrows: n,
                ncols: n,
                indptr: (0..=n).collect(),
                indices: (0..n).collect(),
                data: p.into(),
            }
        };

        // Now we can solve!
        let mut solver = Problem::new(&p_matrix, &q, &a_matrix, &lb, &ub, &self.0)
            .expect("unable to setup problem");
        solver.warm_start_x(&vec![0.0; n]);
        let status = solver.solve();
        if let Status::Solved(solution) = status {
            // We get the raw optimization output
            // TODO: make a determination on whether the price should actually be None

            let mut trade: Map<ProductId, f64> =
                products.iter().map(|(id, _)| (id.clone(), 0.0)).collect();

            let prices: Map<ProductId, f64> = products
                .into_iter()
                .zip(solution.y())
                .map(|((p, _), x)| (p, *x))
                .collect();

            let auth_outcomes: Map<AuthId, AuthOutcome> = auction
                .into_iter()
                .flat_map(|(_, submission)| submission.auths.iter())
                .zip(solution.x())
                .map(|((id, auth), x)| {
                    let mut price = 0.0;
                    for (product_id, weight) in auth.portfolio.iter() {
                        trade[product_id] += (weight * x).abs();
                        price += weight * prices[product_id];
                    }
                    (id.clone(), AuthOutcome { price, trade: *x })
                })
                .collect();

            // This whole bit is somewhat hacky for now. Computing the volume is easy:
            // just add the absolute value of each trade (weighted by the portfolio weight)
            // and ultimately divide by 2. If we had exact arithmetic, then if volume = 0
            // we know the price should be None. However, as we are in floating point land,
            // the solver will arrive at an arbitrary price. It would likely be better to either
            // detect this and say the price is non-existent, OR choose a price according to some
            // criteria that is compatible with no-trade. The latter would require a secondary
            // solve that necessarily distinguishes between zero and non-zero dual variables from
            // the first solve, which is also asking for a world of trouble.
            AuctionOutcome {
                outcomes: auth_outcomes.into_iter().fold(
                    Map::default(),
                    |mut outcomes, (auth_id, auth_outcome)| {
                        // ASSERTION: the auths map will contain a record for every auth
                        let bidder_id = auths.get(&auth_id).unwrap().clone();
                        outcomes
                            .entry(bidder_id)
                            .or_default()
                            .auths
                            .insert(auth_id, auth_outcome);
                        outcomes
                    },
                ),
                products: prices
                    .into_iter()
                    .zip(trade)
                    // ASSERTION: the product_id from either pair is necessarily the same, due to insertion order
                    // guarantees of IndexMap.
                    .map(|((product_id, price), (_, trade))| {
                        (
                            product_id,
                            ProductOutcome {
                                price,
                                trade: trade / 2.0,
                            },
                        )
                    })
                    .collect(),
            }
        } else {
            panic!("inaccurate solve");
        }
    }
}
