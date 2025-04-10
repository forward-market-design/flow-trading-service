use crate::{
    AuctionOutcome, Auth, AuthOutcome, Constant, Point, ProductOutcome, Solver, Submission,
};
use crate::{HashMap, SubmissionOutcome};
use clarabel::{algebra::*, solver::*};
use std::hash::Hash;

/// A solver implementation that uses the Clarabel interior point method
/// for quadratic programming to solve the market clearing problem.
///
/// This solver is generally more accurate but can be slower than ADMM-based
/// solvers for large problems. It's a good choice when high precision is needed.
pub struct ClarabelSolver(DefaultSettings<f64>);

impl Default for ClarabelSolver {
    fn default() -> Self {
        let mut settings = DefaultSettings::default();
        settings.verbose = false;
        Self(settings)
    }
}

impl Solver for ClarabelSolver {
    type Settings = DefaultSettings<f64>;

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

        // Clarabel handles constraints via a cone specification, e.g. Ax + s = b, where s is a cone.
        // The first `nzero` of b and s are just =0, so we do that work upfront.
        let mut b = vec![0.0; nzero];
        let mut s = vec![ZeroConeT(nzero)];

        // Clarabel's matrix input is in the form of CSC, so we handle the memory representation
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
            ) in submission.auths_active.iter()
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
                    .costs_curve
                    .iter()
                    .map(|(group, _)| group)
                    .chain(submission.costs_constant.iter().map(|(group, _)| group))
                {
                    if let Some(x) = group.get(auth_id) {
                        a_nzval.push(x);
                        a_rowval.push(group_offset);
                        group_offset += 1;
                    }
                }

                // Now we add the box constraints. Note that here, we are dynamically
                // growing the constraint vector b and using that to track our row indices.
                // The signs on the lower bound are wonky because we have to use s>=0 as
                // the cone specification.
                if min_trade.is_finite() {
                    a_nzval.push(-1.0);
                    a_rowval.push(b.len());
                    b.push(-min_trade);
                }
                if max_trade.is_finite() {
                    a_nzval.push(1.0);
                    a_rowval.push(b.len());
                    b.push(*max_trade)
                }
            }
        }

        // Now we setup the segment variables
        group_offset = products.len();
        for (_, submission) in auction.into_iter() {
            for (_, curve) in submission.costs_curve.iter() {
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
                    // x0 <= y <= x1 ==> -y + s == -x0 and y + s == x1
                    a_nzval.push(-1.0);
                    a_rowval.push(b.len());
                    b.push(-x0);
                    a_nzval.push(1.0);
                    a_rowval.push(b.len());
                    b.push(x1);
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
            ) in submission.costs_constant.iter()
            {
                // Constant segments contribute nothing to the quadratic objective, but otherwise
                // act a lot like a curve.
                p.push(0.0);
                q.push(-price);

                // Insert a new column
                a_colptr.push(a_nzval.len());

                // Ensure it counts towards the group
                a_nzval.push(-1.0);
                a_rowval.push(group_offset);

                // Also establish the relevant constraints
                if min.is_finite() {
                    a_nzval.push(-1.0);
                    a_rowval.push(b.len());
                    b.push(-min);
                }
                if max.is_finite() {
                    a_nzval.push(1.0);
                    a_rowval.push(b.len());
                    b.push(*max);
                }

                // Advance the group offset for the next bid/constraint
                group_offset += 1;
            }
        }

        // We need to polish off the CSC matrix
        a_colptr.push(a_nzval.len());

        let a_matrix = CscMatrix {
            m: b.len(),
            n: p.len(),
            colptr: a_colptr,
            rowval: a_rowval,
            nzval: a_nzval,
        };

        assert!(a_matrix.check_format().is_ok()); // TODO: maybe remove this

        // We also need to cleanup the cone specification
        s.push(NonnegativeConeT(b.len() - nzero));

        // Finally, we need to convert our p spec into a csc matrix
        let p_matrix = {
            let m = p.len();
            let n = p.len();

            CscMatrix {
                m,
                n,
                colptr: (0..=n).collect(),
                rowval: (0..n).collect(),
                nzval: p,
            }
        };

        // Now we can solve!
        let mut solver = DefaultSolver::new(&p_matrix, &q, &a_matrix, &b, &s, self.0.clone());
        solver.solve();
        assert_eq!(solver.solution.status, SolverStatus::Solved);

        // We get the raw optimization output
        // TODO: make a determination on whether the price should actually be None

        let mut trade: HashMap<ProductId, f64> =
            products.iter().map(|(id, _)| (id.clone(), 0.0)).collect();

        let prices: HashMap<ProductId, f64> = products
            .into_iter()
            .zip(solver.solution.z.iter())
            .map(|((p, _), x)| (p, *x))
            .collect();

        let auth_outcomes: HashMap<AuthId, AuthOutcome> = auction
            .into_iter()
            .flat_map(|(_, submission)| submission.auths_active.iter())
            .zip(solver.solution.x.iter())
            .map(|((id, auth), x)| {
                let mut price = 0.0;
                for (product_id, weight) in auth.portfolio.iter() {
                    // TODO: we're adding floats, which has a possibility of precision loss.
                    // This probably shouldn't matter, but if we learn that it does we can easily
                    // use a Kahan-style summation here instead.
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
        // the first solve, which is also asking for a world of trouble. For now, we just take
        // whatever the solver returns as the price without further analysis, though we should
        // consider constructing an auxiliary optimization program to specifically force
        // a particular price when is a freedom.

        // Roll up the decision variable solutions to the bidder level
        let mut bidders = HashMap::<BidderId, SubmissionOutcome<AuthId>>::default();
        for (auth_id, auth_outcome) in auth_outcomes {
            // ASSERTION: the auths map will contain a record for every auth
            let bidder_id = auths.get(&auth_id).unwrap().clone();
            bidders
                .entry(bidder_id)
                .or_default()
                .auths
                .insert(auth_id, auth_outcome);
        }

        // Patch the results to also include the inactive auths. Note that there is a possibility
        // that such portfolio prices might not exist, since the underlying product(s) are not
        // guaranteed to have been traded.
        for (bidder_id, submission) in auction {
            for (auth_id, portfolio) in submission.auths_inactive.iter() {
                bidders.entry(bidder_id.clone()).or_default().auths.insert(
                    auth_id.clone(),
                    AuthOutcome {
                        price: portfolio.iter().fold(0.0, |sum, (product_id, weight)| {
                            sum + weight
                                * prices.get(product_id).map(Clone::clone).unwrap_or(f64::NAN)
                        }),
                        trade: 0.0,
                    },
                );
            }
        }

        // Generate per-product outcomes related to trade volume and price
        let products = prices
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
            .collect();

        AuctionOutcome { bidders, products }
    }
}
