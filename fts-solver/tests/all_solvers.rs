#![allow(unused_macros)]
use rstest_reuse::template;

// This creates a testing "template" to allow for the injection of each solver
// implementation

#[template]
#[rstest]
#[case::clarabel(fts_solver::clarabel::ClarabelSolver::default())]
#[case::osqp(fts_solver::osqp::OsqpSolver::default())]
pub fn all_solvers<AuthId, ProductId>(#[case] solver: impl solver::Solver) -> () {}
