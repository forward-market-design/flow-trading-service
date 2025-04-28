use clap::ValueEnum;
use fts_solver::{
    AuctionOutcome, Solver as _, Submission,
    clarabel::ClarabelSolver,
    io::{BidderId, PortfolioId, ProductId},
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
    pub fn solve<T>(&self, auction: &T) -> AuctionOutcome<BidderId, PortfolioId, ProductId>
    where
        for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<PortfolioId, ProductId>)>,
    {
        match self {
            SolverLib::Clarabel => {
                let solver = ClarabelSolver::default();
                solver.solve(auction).expect("could not solve auction")
            }
            SolverLib::Osqp => {
                let solver = OsqpSolver::default();
                solver.solve(auction).expect("could not solve auction")
            }
        }
    }
}
