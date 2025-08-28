use approx::assert_abs_diff_eq;
use fts_solver::io::{Auction, DemandId, Outcome, PortfolioId, ProductId};
use rstest::*;
use rstest_reuse::{self, *};
use serde::de::DeserializeOwned;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
};

// This creates a testing "template" to allow for the injection of each solver
// implementation

#[template]
#[rstest]
#[case::clarabel(fts_solver::clarabel::ClarabelSolver::default())]
#[case::osqp(fts_solver::osqp::OsqpSolver::default())]
pub fn all_solvers<PortfolioId, ProductId>(
    #[case] solver: impl solver::Solver<
        PortfolioOutcome = fts_solver::PortfolioOutcome,
        ProductOutcome = fts_solver::ProductOutcome,
    >,
) -> () {
}

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
#[tokio::test]
async fn run_auction<
    T: fts_core::ports::Solver<
            DemandId,
            PortfolioId,
            ProductId,
            PortfolioOutcome = (),
            ProductOutcome = (),
        >,
>(
    solver: T,
    #[files("tests/samples/**/output.json")] output: PathBuf,
) where
    T::PortfolioOutcome: DeserializeOwned,
    T::ProductOutcome: DeserializeOwned,
{
    use fts_solver::io::Outcome;

    let mut input = output.clone();
    input.set_file_name("input.json");

    let auction: Auction =
        serde_json::from_reader(BufReader::new(File::open(input).unwrap())).unwrap();

    let reference: Outcome<T::PortfolioOutcome, T::ProductOutcome> =
        serde_json::from_reader(BufReader::new(File::open(output).unwrap())).unwrap();

    let solution = auction.solve(solver).await;

    cmp(&solution, &reference, 1e-6, 1e-6);
}

#[rstest]
fn check_mps_export(#[files("tests/samples/**/export.mps")] output: PathBuf) {
    let mut input = output.clone();
    input.set_file_name("input.json");

    let auction: Auction =
        serde_json::from_reader(BufReader::new(File::open(input).unwrap())).unwrap();

    let mut output_bytes = Vec::new();
    let output_size = File::open(output)
        .unwrap()
        .read_to_end(&mut output_bytes)
        .unwrap();
    let mut export_bytes = Vec::with_capacity(output_size);
    auction.export_mps(&mut export_bytes).unwrap();
    assert!(output_bytes == export_bytes, "mps files are not identical");
}

#[rstest]
fn check_lp_export(#[files("tests/samples/**/export.lp")] output: PathBuf) {
    let mut input = output.clone();
    input.set_file_name("input.json");

    let auction: Auction =
        serde_json::from_reader(BufReader::new(File::open(input).unwrap())).unwrap();

    let mut output_bytes = Vec::new();
    let output_size = File::open(output)
        .unwrap()
        .read_to_end(&mut output_bytes)
        .unwrap();
    let mut export_bytes = Vec::with_capacity(output_size);
    auction.export_lp(&mut export_bytes).unwrap();
    assert!(output_bytes == export_bytes, "lp files are not identical");
}

fn cmp(a: &Outcome<(), ()>, b: &Outcome<(), ()>, qeps: f64, peps: f64) {
    assert_eq!(a.products.len(), b.products.len());
    for ((p1, o1), (p2, o2)) in a.products.iter().zip(b.products.iter()) {
        assert_eq!(p1, p2);
        assert_abs_diff_eq!(o1.trade, o2.trade, epsilon = qeps);
        assert_abs_diff_eq!(o1.price.unwrap(), o2.price.unwrap(), epsilon = peps);
    }

    assert_eq!(a.portfolios.len(), b.portfolios.len());

    for ((p1, o1), (p2, o2)) in a.portfolios.iter().zip(b.portfolios.iter()) {
        assert_eq!(p1, p2);
        assert_abs_diff_eq!(o1.trade, o2.trade, epsilon = qeps);
        assert_abs_diff_eq!(o1.price.unwrap(), o2.price.unwrap(), epsilon = peps);
    }
}
