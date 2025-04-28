use crate::{AuctionOutcome, HashMap, PortfolioOutcome, ProductOutcome, Solver, Submission};
use osqp::{CscMatrix, Problem, Settings, Solution, Status};
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
        Self(Settings::default().verbose(false).polishing(true))
    }
}

impl Solver for OsqpSolver {
    type Settings = Settings;
    type Status = OsqpStatus;

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
    ) -> Result<AuctionOutcome<BidderId, AuthId, ProductId>, Self::Status>
    where
        for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<AuthId, ProductId>)>,
    {
        let (products, ncosts) = super::prepare(auction);

        if products.len() == 0 {
            return Ok(AuctionOutcome::default());
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
            for (id, portfolio) in submission.portfolios.iter() {
                // portfolio variables contribute nothing to the objective
                p.push(0.0);
                q.push(0.0);

                // start a new column in the constraint matrix
                a_colptr.push(a_nzval.len());

                // We copy the portfolio definition into the matrix
                for (product, &weight) in portfolio.iter() {
                    a_nzval.push(weight);
                    a_rowval.push(products.get_index_of(product).unwrap());
                }

                // Now we embed the weights from each group. This loop is a little wonky;
                // in matrix terms, the representation is transposed in the wrong way.
                // However, we expect the number of groups to be fairly small, so simply
                // searching every group for every portfolio in the submission is not so bad.
                for (offset, (group, _)) in submission.demand_curves.iter().enumerate() {
                    if let Some(&weight) = group.get(id) {
                        a_nzval.push(weight);
                        a_rowval.push(group_offset + offset);
                    }
                }
            }

            group_offset += submission.demand_curves.len();
        }

        // Now we setup the segment variables
        group_offset = products.len();
        for (_, submission) in auction.into_iter() {
            for (_, curve) in submission.demand_curves.iter() {
                for segment in curve.iter() {
                    let (m, pzero) = segment.slope_intercept();

                    // Setup the contributions to the objective
                    p.push(-m);
                    q.push(-pzero);

                    // Insert a new column
                    a_colptr.push(a_nzval.len());

                    // Ensure it counts towards the group
                    a_nzval.push(-1.0);
                    a_rowval.push(group_offset);

                    // Setup the box constraints
                    a_nzval.push(1.0);
                    a_rowval.push(lb.len());
                    lb.push(segment.q0);
                    ub.push(segment.q1);
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
        let (status, solution) = remap(solver.solve());

        if status.ok() {
            // Does not panic, because ok() is only true when we return the solution
            let solution = solution.unwrap();
            // We get the raw optimization output

            let mut product_outcomes: HashMap<ProductId, ProductOutcome> = products
                .into_iter()
                .zip(solution.y())
                .map(|(product, &price)| (product, ProductOutcome { price, trade: 0.0 }))
                .collect();

            let mut trades = solution.x().iter();

            let submission_outcomes = auction
                .into_iter()
                .map(|(bidder_id, submission)| {
                    let outcome = submission
                        .portfolios
                        .iter()
                        .map(|(id, portfolio)| {
                            // Safe, because we necessarily have every portfolio represented in the solution
                            let trade = *trades.next().unwrap();

                            let mut price = 0.0;

                            for (product_id, weight) in portfolio.iter() {
                                // Safe, because product outcomes contains all referenced products
                                let product_outcome = product_outcomes.get_mut(product_id).unwrap();

                                // We report the trade (in an absolute sense), to be halved later
                                product_outcome.trade += (weight * trade).abs();

                                // We also compute the effective price of the portfolio
                                price += weight * product_outcome.price;

                                // TODO: consider special summation algorithms (i.e. Kahan) for the above dot products
                            }

                            (id.clone(), PortfolioOutcome { price, trade })
                        })
                        .collect::<HashMap<_, _>>();

                    (bidder_id.clone(), outcome)
                })
                .collect::<HashMap<_, _>>();

            // We have double-counted the trade for each product, so we halve it
            for outcome in product_outcomes.values_mut() {
                outcome.trade *= 0.5;
            }
            // TODO:
            // We have assigned the products prices straight from the solver
            // (and computed the portfolio prices from those).
            // Under pathological circumstances, the price may not be unique
            // (either when there is no trade, or the supply exactly matches the demand).
            // We should think about injecting an auxiliary solve for choosing a canonical
            // price and/or for detecting when there is such a degeneracy.

            // TODO:
            // When there are "flat" demand curves, it is possible for nonuniqueness
            // in the traded outcomes. The convex regularization is to minimize the L2 norm
            // of the trades as a tie-break. We should think about the best way to regularize
            // the solve accordingly.

            Ok(AuctionOutcome {
                submissions: submission_outcomes,
                products: product_outcomes,
            })
        } else {
            Err(status)
        }
    }
}

/// OSQP's status contains a reference to the solution data,
/// which makes the lifetimes complicated. Instead, we just
/// mirror the names and call it a day.
#[derive(Clone, Copy, Debug)]
pub enum OsqpStatus {
    /// Solved
    Solved,
    /// Solved, but less accurately
    SolvedInaccurate,
    /// Maybe solveable, but needs more iterations
    MaxIterationsReached,
    /// Maybe solveable, but needs more time
    TimeLimitReached,
    /// Impossible
    PrimalInfeasible,
    /// Impossible
    PrimalInfeasibleInaccurate,
    /// Impossible
    DualInfeasible,
    /// Impossible
    DualInfeasibleInaccurate,
    /// Impossible
    NonConvex,
}

impl OsqpStatus {
    fn ok(self) -> bool {
        match self {
            Self::Solved => true,
            Self::SolvedInaccurate => {
                tracing::warn!(status = ?self, "convergence issues");
                true
            }
            _ => false,
        }
    }
}

fn remap<'a>(value: Status<'a>) -> (OsqpStatus, Option<Solution<'a>>) {
    match value {
        Status::Solved(x) => (OsqpStatus::Solved, Some(x)),
        Status::SolvedInaccurate(x) => (OsqpStatus::SolvedInaccurate, Some(x)),
        Status::MaxIterationsReached(x) => (OsqpStatus::MaxIterationsReached, Some(x)),
        Status::TimeLimitReached(x) => (OsqpStatus::TimeLimitReached, Some(x)),
        Status::PrimalInfeasible(_) => (OsqpStatus::PrimalInfeasible, None),
        Status::PrimalInfeasibleInaccurate(_) => (OsqpStatus::PrimalInfeasibleInaccurate, None),
        Status::DualInfeasible(_) => (OsqpStatus::DualInfeasible, None),
        Status::DualInfeasibleInaccurate(_) => (OsqpStatus::DualInfeasibleInaccurate, None),
        Status::NonConvex(_) => (OsqpStatus::NonConvex, None),
        _ => panic!("unknown status"),
    }
}
