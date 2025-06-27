use crate::{PortfolioOutcome, ProductOutcome, disaggregate};
use fts_core::{
    models::{DemandCurve, Map},
    ports::Solver,
};
use osqp::{CscMatrix, Problem, Settings, Solution, Status};
use std::{hash::Hash, marker::PhantomData};

/// A solver implementation that uses the OSQP (Operator Splitting Quadratic Program)
/// solver to find market clearing prices and trades.
///
/// OSQP uses the Alternating Direction Method of Multipliers (ADMM) approach,
/// which can be faster than interior point methods for large-scale problems,
/// though sometimes with lower precision.
pub struct OsqpSolver<DemandId, PortfolioId, ProductId>(
    Settings,
    PhantomData<(DemandId, PortfolioId, ProductId)>,
);

impl<A, B, C> OsqpSolver<A, B, C> {
    /// create a new solver with the given settings
    pub fn new(settings: Settings) -> Self {
        Self(settings, PhantomData::default())
    }
}

impl<A, B, C> Default for OsqpSolver<A, B, C> {
    fn default() -> Self {
        Self(
            Settings::default().verbose(false).polishing(true),
            PhantomData::default(),
        )
    }
}

impl<
    DemandId: Clone + Eq + Hash,
    PortfolioId: Clone + Eq + Hash,
    ProductId: Clone + Eq + Hash + Ord,
> OsqpSolver<DemandId, PortfolioId, ProductId>
{
    fn solve(
        settings: Settings,
        demand_curves: Map<DemandId, DemandCurve>,
        portfolios: Map<PortfolioId, (Map<DemandId>, Map<ProductId>)>,
    ) -> Result<
        (
            Map<PortfolioId, PortfolioOutcome>,
            Map<ProductId, ProductOutcome>,
        ),
        OsqpStatus,
    > {
        // This prepare method canonicalizes the input in a manner appropriate for naive CSC construction
        let (demand_curves, portfolios, mut portfolio_outcomes, mut product_outcomes) =
            super::prepare(demand_curves, portfolios);

        // If there are no portfolios or products, there is nothing to do.
        if portfolio_outcomes.len() == 0 || product_outcomes.len() == 0 {
            return Ok((portfolio_outcomes, product_outcomes));
        }

        // The trade and bid constraints are all (something) = 0, we need to
        // know how many of these there are in order to handle the box
        // constraints for each decision variable
        let nproducts = product_outcomes.len();
        let ndemands = demand_curves.len();

        // Our quadratic term is diagonal, so we build the matrix by defining its diagonal
        let mut p = Vec::new();
        // and these are the linear terms
        let mut q = Vec::new();

        // OSQP handles constraints via a box specification, e.g. lb <= Ax <= ub,
        // where equality is handled via setting lb[i] = ub[i].
        // The first `nzero` of lb and ub are =0 so we do that work upfront.
        let mut lb = vec![0.0; nproducts + ndemands];
        let mut ub = vec![0.0; nproducts + ndemands];

        // OSQP's matrix input is in the form of CSC, so we handle the memory representation
        // carefully.
        let mut a_nzval = Vec::new();
        let mut a_rowval = Vec::new();
        let mut a_colptr = Vec::new();

        // We begin by setting up the portfolio variables.
        for (demand_group, product_group) in portfolios.values() {
            // We can skip any portfolio variable that does not have associated products or demands
            if product_group.len() == 0 || demand_group.len() == 0 {
                continue;
            }

            // portfolio variables contribute nothing to the objective
            p.push(0.0);
            q.push(0.0);

            // start a new column in the constraint matrix
            a_colptr.push(a_nzval.len());

            // We copy the product weights into the matrix
            for (product_id, &weight) in product_group.iter() {
                // SAFETY: this unwrap() is guaranteed by the logic in prepare()
                let idx = product_outcomes.get_index_of(product_id).unwrap();
                a_nzval.push(weight);
                a_rowval.push(idx);
            }

            // We copy the demand weights into the matrix as well
            for (demand_id, &weight) in demand_group.iter() {
                // SAFETY: this unwrap() is guaranteed by the logic in prepare()
                let idx = demand_curves.get_index_of(demand_id).unwrap();
                a_nzval.push(weight);
                a_rowval.push(nproducts + idx);
            }
        }

        // Now we setup the segment variables
        for (offset, (_, demand_curve)) in demand_curves.into_iter().enumerate() {
            let row = nproducts + offset;
            let (min, max) = demand_curve.domain();
            let points = demand_curve.points();

            if let Some(segments) = disaggregate(points.into_iter(), min, max) {
                for segment in segments {
                    // TODO: propagate the error upwards
                    let segment = segment.unwrap();
                    let (m, pzero) = segment.slope_intercept();

                    // Setup the contributions to the objective
                    p.push(-m);
                    q.push(-pzero);

                    // Insert a new column
                    a_colptr.push(a_nzval.len());

                    // Ensure it counts towards the group
                    a_nzval.push(-1.0);
                    a_rowval.push(row);

                    // Setup the box constraints
                    a_nzval.push(1.0);
                    a_rowval.push(lb.len());
                    lb.push(segment.q0);
                    ub.push(segment.q1);
                }
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
        let mut solver = Problem::new(&p_matrix, &q, &a_matrix, &lb, &ub, &settings)
            .expect("unable to setup problem");
        solver.warm_start_x(&vec![0.0; n]);
        let (status, solution) = remap(solver.solve());

        if status.ok() {
            // Does not panic, because ok() is only true when we return the solution
            let solution = solution.unwrap();
            // We get the raw optimization output

            // Now we copy the solution back
            super::finalize(
                solution.x().iter(),
                solution.y().iter(),
                &portfolios,
                &mut portfolio_outcomes,
                &mut product_outcomes,
            );

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

            Ok((portfolio_outcomes, product_outcomes))
        } else {
            Err(status)
        }
    }
}

impl<
    DemandId: Clone + Eq + Hash + Ord + Send + Sync + 'static,
    PortfolioId: Clone + Eq + Hash + Ord + Send + Sync + 'static,
    ProductId: Clone + Eq + Hash + Ord + Send + Sync + 'static,
> Solver<DemandId, PortfolioId, ProductId> for OsqpSolver<DemandId, PortfolioId, ProductId>
{
    type Error = tokio::task::JoinError;
    type PortfolioOutcome = PortfolioOutcome;
    type ProductOutcome = ProductOutcome;

    type State = Option<Map<PortfolioId>>;

    async fn solve(
        &self,
        demand_curves: Map<DemandId, DemandCurve>,
        portfolios: Map<PortfolioId, (Map<DemandId>, Map<ProductId>)>,
        _state: Self::State,
    ) -> Result<
        (
            Map<PortfolioId, Self::PortfolioOutcome>,
            Map<ProductId, Self::ProductOutcome>,
        ),
        Self::Error,
    > {
        let settings = self.0.clone();
        let solution =
            tokio::spawn(async move { Self::solve(settings, demand_curves, portfolios) }).await?;

        // TODO: The JoinError happens when we panic inside.
        // We can change this later, for now we just assume the solver worked.
        Ok(solution.expect("failed to solve"))
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
