use fts_solver::{
    AuctionOutcome,
    cli::{BidderId, PortfolioId, ProductId, RawAuction},
};
use rstest::*;
use rstest_reuse::{self, *};
use std::{fmt::Debug, fs::File, io::BufReader, path::PathBuf};

mod all_solvers;
use all_solvers::all_solvers;

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
        assert!((o1.trade - o2.trade).abs() <= qeps);
        assert!((o1.price - o2.price).abs() <= peps);
    }

    assert_eq!(a.submissions.len(), b.submissions.len());
    for ((b1, s1), (b2, s2)) in a.submissions.iter().zip(b.submissions.iter()) {
        assert_eq!(b1, b2);
        assert_eq!(s1.len(), s2.len());

        for ((p1, o1), (p2, o2)) in s1.iter().zip(s2.iter()) {
            assert_eq!(p1, p2);
            assert!((o1.trade - o2.trade).abs() <= qeps);
            assert!((o1.price - o2.price).abs() <= peps);
        }
    }
}
