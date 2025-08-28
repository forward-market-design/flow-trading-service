use crate::disaggregate;
use clarabel::{algebra::*, solver::*};
use fts_core::{
    models::{Basis, DemandCurve, Map, Weights},
    ports::{Outcome, Solver},
};
use std::{hash::Hash, marker::PhantomData};

/// A solver implementation that uses the Clarabel interior point method
/// for quadratic programming to solve the market clearing problem.
///
/// This solver is generally more accurate but can be slower than ADMM-based
/// solvers for large problems. It's a good choice when high precision is needed.
pub struct ClarabelSolver<DemandId, PortfolioId, ProductId>(
    DefaultSettings<f64>,
    PhantomData<(DemandId, PortfolioId, ProductId)>,
);

impl<A, B, C> ClarabelSolver<A, B, C> {
    /// create a new solver with the given settings
    pub fn new(settings: DefaultSettings<f64>) -> Self {
        Self(settings, PhantomData::default())
    }
}

impl<A, B, C> Default for ClarabelSolver<A, B, C> {
    fn default() -> Self {
        let mut settings = DefaultSettings::default();
        settings.verbose = false;
        Self(settings, PhantomData::default())
    }
}

impl<
    DemandId: Clone + Eq + Hash,
    PortfolioId: Clone + Eq + Hash,
    ProductId: Clone + Eq + Hash + Ord,
> ClarabelSolver<DemandId, PortfolioId, ProductId>
{
    fn solve(
        settings: DefaultSettings<f64>,
        demand_curves: Map<DemandId, DemandCurve>,
        portfolios: Map<PortfolioId, (Weights<DemandId>, Basis<ProductId>)>,
    ) -> Result<(Map<PortfolioId, Outcome<()>>, Map<ProductId, Outcome<()>>), SolverStatus> {
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

        // Clarabel handles constraints via a cone specification, e.g. Ax + s = b, where s is a cone.
        // The first `nzero` of b and s are just =0, so we do that work upfront.
        let mut b = vec![0.0; nproducts + ndemands];
        let mut s = vec![ZeroConeT(b.len()), NonnegativeConeT(0)];

        // Clarabel's matrix input is in the form of CSC, so we handle the memory representation
        // carefully.
        let mut a_nzval = Vec::new();
        let mut a_rowval = Vec::new();
        let mut a_colptr = Vec::new();

        // We begin by setting up the portfolio variables.
        for (demand, basis) in portfolios.values() {
            // We can skip any portfolio variable that does not have associated products or demands
            // (This is because our outcomes are preloaded with zero solutions)
            if basis.len() == 0 || demand.len() == 0 {
                continue;
            }

            // portfolio variables contribute nothing to the objective
            p.push(0.0);
            q.push(0.0);

            // start a new column in the constraint matrix
            a_colptr.push(a_nzval.len());

            // We copy the product weights into the matrix
            for (product_id, &weight) in basis.iter() {
                // SAFETY: this unwrap() is guaranteed by the logic in prepare()
                let idx = product_outcomes.get_index_of(product_id).unwrap();
                a_nzval.push(weight);
                a_rowval.push(idx);
            }

            // We copy the demand weights into the matrix as well
            for (demand_id, &weight) in demand.iter() {
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
                    // x0 <= y <= x1 ==> -y + s == -x0 and y + s == x1
                    if segment.q0.is_finite() {
                        a_nzval.push(-1.0);
                        a_rowval.push(b.len());
                        b.push(-segment.q0);
                    }
                    if segment.q1.is_finite() {
                        a_nzval.push(1.0);
                        a_rowval.push(b.len());
                        b.push(segment.q1);
                    }
                }
            }
        }

        // We need to polish off the CSC matrix
        a_colptr.push(a_nzval.len());

        let m = b.len();
        let n = p.len();

        let a_matrix = CscMatrix {
            m,
            n,
            colptr: a_colptr,
            rowval: a_rowval,
            nzval: a_nzval,
        };

        assert!(a_matrix.check_format().is_ok()); // TODO: maybe remove this

        // We also need to cleanup the cone specification
        s[1] = NonnegativeConeT(b.len() - nproducts - ndemands);

        // Finally, we need to convert our p spec into a csc matrix
        let p_matrix = {
            CscMatrix {
                m: n,
                n,
                colptr: (0..=n).collect(),
                rowval: (0..n).collect(),
                nzval: p,
            }
        };

        // Now we can solve!
        let mut solver = DefaultSolver::new(&p_matrix, &q, &a_matrix, &b, &s, settings)
            .expect("valid solver config");
        solver.solve();
        match solver.solution.status {
            SolverStatus::Solved => {}
            SolverStatus::AlmostSolved => {
                tracing::warn!(status = ?solver.solution.status, "convergence issues");
            }
            status => {
                return Err(status);
            }
        };

        // Now we copy the solution back
        super::finalize(
            solver.solution.x.iter(),
            solver.solution.z.iter(),
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
    }
}

impl<
    DemandId: Clone + Eq + Hash + Ord + Send + Sync + 'static,
    PortfolioId: Clone + Eq + Hash + Ord + Send + Sync + 'static,
    ProductId: Clone + Eq + Hash + Ord + Send + Sync + 'static,
> Solver<DemandId, PortfolioId, ProductId> for ClarabelSolver<DemandId, PortfolioId, ProductId>
{
    type Error = tokio::task::JoinError;
    type PortfolioOutcome = ();
    type ProductOutcome = ();
    type State = ();

    async fn solve(
        &self,
        demand_curves: Map<DemandId, DemandCurve>,
        portfolios: Map<PortfolioId, (Weights<DemandId>, Basis<ProductId>)>,
        _state: Self::State,
    ) -> Result<
        (
            Map<PortfolioId, Outcome<Self::PortfolioOutcome>>,
            Map<ProductId, Outcome<Self::ProductOutcome>>,
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
