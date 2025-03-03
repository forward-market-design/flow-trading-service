use crate::{
    AuctionOutcome, Auth, AuthOutcome, Constant, Point, ProductOutcome, Solver, Submission,
};
use crate::{Map, Set};
use clarabel::{algebra::*, solver::*};
use std::hash::Hash;

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

    fn solve<AuthId: Eq + Hash + Clone + Ord, ProductId: Eq + Hash + Clone + Ord>(
        &self,
        auction: &[Submission<AuthId, ProductId>],
    ) -> AuctionOutcome<AuthId, ProductId> {
        if auction.is_empty() {
            return AuctionOutcome {
                auths: Default::default(),
                products: Default::default(),
            };
        }

        // In order to setup the the optimization program, we need to define
        // up front the full space of products, as well as assign a canonical
        // enumerative index to each of them.
        let products = {
            // Gather the set of products from every authorized portfolio
            let mut products = auction
                .iter()
                .flat_map(|submission| {
                    submission
                        .auths
                        .values()
                        .flat_map(|auth| auth.portfolio.keys())
                        .map(|id| id.clone())
                })
                .collect::<Set<ProductId>>();

            // Provide a canonical ordering to the product ids
            products.sort_unstable();

            // Build the index lookup
            products
                .into_iter()
                .enumerate()
                .map(|(a, b)| (b, a))
                .collect::<Map<ProductId, usize>>()
        };

        // The trade and bid constraints are all (something) = 0, we need to
        // know how many of these there are in order to handle the box
        // constraints for each decision variable
        let nzero = products.len()
            + auction
                .iter()
                .map(|submission| submission.cost_curves.len() + submission.cost_constants.len())
                .sum::<usize>();

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
        for submission in auction.iter() {
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
        for submission in auction.iter() {
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
            ) in submission.cost_constants.iter()
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

        let mut volume: Map<ProductId, f64> =
            products.iter().map(|(id, _)| (id.clone(), 0.0)).collect();

        let prices: Map<ProductId, f64> = products
            .into_iter()
            .zip(solver.solution.z.iter())
            .map(|((p, _), x)| (p, *x))
            .collect();

        let auth_outcomes: Map<AuthId, AuthOutcome> = auction
            .iter()
            .flat_map(|submission| submission.auths.iter())
            .zip(solver.solution.x.iter())
            .map(|((id, auth), x)| {
                let mut price = 0.0;
                for (product_id, weight) in auth.portfolio.iter() {
                    // TODO: we're adding floats, which has a possibility of precision loss.
                    // This probably shouldn't matter, but if we learn that it does we can easily
                    // use a Kahan-style summation here instead.
                    volume[product_id] += (weight * x).abs();
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
        AuctionOutcome {
            auths: auth_outcomes,
            products: prices
                .into_iter()
                .zip(volume)
                .map(|((product_id, price), (_, volume))| {
                    (
                        product_id,
                        ProductOutcome {
                            price,
                            volume: volume / 2.0,
                        },
                    )
                })
                .collect(),
        }
    }
}
