use approx::assert_abs_diff_eq;
use fts_solver::{
    Auction, AuctionOutcome,
    export::{export_lp, export_mps},
    io::AuctionDto,
};
use rstest::*;
use rstest_reuse::{self, *};
use std::{
    fmt::Debug,
    fs::File,
    io::{BufReader, Read},
    ops::Deref,
    path::PathBuf,
};

// This creates a testing "template" to allow for the injection of each solver
// implementation

#[template]
#[rstest]
#[case::clarabel(fts_solver::clarabel::ClarabelSolver::default())]
#[case::osqp(fts_solver::osqp::OsqpSolver::default())]
pub fn all_solvers<PortfolioId, ProductId>(#[case] solver: impl solver::Solver) -> () {}

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
    #[files("tests/samples/**/output.json")] output: PathBuf,
) {
    let mut input = output.clone();
    input.set_file_name("input.json");

    let raw_auction: AuctionDto =
        serde_json::from_reader(BufReader::new(File::open(input).unwrap())).unwrap();

    let auction: Auction<_, _, _> = raw_auction.try_into().unwrap();

    let reference: AuctionOutcome<_, _, _> =
        serde_json::from_reader(BufReader::new(File::open(output).unwrap())).unwrap();

    let solution = solver.solve(auction.deref());

    cmp(&solution, &reference, 1e-6, 1e-6);
}

#[rstest]
fn check_mps_export(#[files("tests/samples/**/export.mps")] output: PathBuf) {
    let mut input = output.clone();
    input.set_file_name("input.json");

    let raw_auction: AuctionDto =
        serde_json::from_reader(BufReader::new(File::open(input).unwrap())).unwrap();

    let auction: Auction<_, _, _> = raw_auction.try_into().unwrap();

    let mut output_bytes = Vec::new();
    let output_size = File::open(output)
        .unwrap()
        .read_to_end(&mut output_bytes)
        .unwrap();
    let mut export_bytes = Vec::with_capacity(output_size);
    export_mps(auction.deref(), &mut export_bytes).unwrap();
    assert!(output_bytes == export_bytes, "mps files are not identical");
}

#[rstest]
fn check_lp_export(#[files("tests/samples/**/export.lp")] output: PathBuf) {
    let mut input = output.clone();
    input.set_file_name("input.json");

    let raw_auction: AuctionDto =
        serde_json::from_reader(BufReader::new(File::open(input).unwrap())).unwrap();

    let auction: Auction<_, _, _> = raw_auction.try_into().unwrap();

    let mut output_bytes = Vec::new();
    let output_size = File::open(output)
        .unwrap()
        .read_to_end(&mut output_bytes)
        .unwrap();
    let mut export_bytes = Vec::with_capacity(output_size);
    export_lp(auction.deref(), &mut export_bytes).unwrap();
    assert!(output_bytes == export_bytes, "mps files are not identical");
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
