use clap::ValueEnum;
use fts_solver::{
    clarabel::ClarabelSolver,
    io::{Auction, Outcome},
    osqp::OsqpSolver,
};

// This explicitly articulates the available solvers for the `solve` subcommand
#[derive(Clone, Copy, ValueEnum)]
pub enum SolverLib {
    Clarabel,
    Osqp,
}

// Conveniently, we can use the same enum to handle the particulars of calling into
// the various solver implementations
impl SolverLib {
    pub async fn solve(&self, auction: Auction) -> Outcome<(), ()> {
        match self {
            SolverLib::Clarabel => auction.solve(ClarabelSolver::default()).await,
            SolverLib::Osqp => auction.solve(OsqpSolver::default()).await,
        }
    }
}
