use approx::assert_abs_diff_eq;
use fts_solver::{
    AuctionOutcome,
    cli::{BidderId, PortfolioId, ProductId, RawAuction},
};
use rstest::*;
use rstest_reuse::{self, *};
use std::{fmt::Debug, fs::File, io::BufReader, path::PathBuf};

mod all_solvers;
use all_solvers::all_solvers;

// This test case is actually a dynamically generated, Cartesian product of test cases.
// For every solver implementation, and for every (input.json, output.json) pair in `./samples/**`,
//   1. Read in the auction input,
//   2. Read in the known-good auction output,
//   3. Solve the auction from the input,
//   4. Compare the solution to the known-good output.
// Comparing the solution is done somewhat simplistically for now, but can be expanded upon later.
// For now, the comparison requires the ordering of products, submissions, and portfolios to be preserved,
// and checks a simple absolute-difference measure against products and portfolios in both their prices and trades.

#[apply(all_solvers)]
#[rstest]
fn run_auction(
    solver: impl fts_solver::Solver,
    #[files("tests/samples/**/input.json")] input: PathBuf,
) {
    let mut output = input.clone();
    output.set_file_name("output.json");

    let auction: RawAuction =
        serde_json::from_reader(BufReader::new(File::open(input).unwrap())).unwrap();

    let reference: AuctionOutcome<BidderId, PortfolioId, ProductId> =
        serde_json::from_reader(BufReader::new(File::open(output).unwrap())).unwrap();

    let solution = solver.solve(&auction.prepare().unwrap());

    cmp(&solution, &reference, 1e-6, 1e-6);
}

fn cmp<BidderId: Debug + Eq, PortfolioId: Debug + Eq, ProductId: Debug + Eq>(
    a: &AuctionOutcome<BidderId, PortfolioId, ProductId>,
    b: &AuctionOutcome<BidderId, PortfolioId, ProductId>,
    qeps: f64,
    peps: f64,
) {
    assert_eq!(a.products.len(), b.products.len());
    for ((p1, o1), (p2, o2)) in a.products.iter().zip(b.products.iter()) {
        assert_eq!(p1, p2);
        assert_abs_diff_eq!(o1.trade, o2.trade, epsilon = qeps);
        assert_abs_diff_eq!(o1.price, o2.price, epsilon = peps);
    }

    assert_eq!(a.submissions.len(), b.submissions.len());
    for ((b1, s1), (b2, s2)) in a.submissions.iter().zip(b.submissions.iter()) {
        assert_eq!(b1, b2);
        assert_eq!(s1.len(), s2.len());

        for ((p1, o1), (p2, o2)) in s1.iter().zip(s2.iter()) {
            assert_eq!(p1, p2);
            assert_abs_diff_eq!(o1.trade, o2.trade, epsilon = qeps);
            assert_abs_diff_eq!(o1.price, o2.price, epsilon = peps);
        }
    }
}
