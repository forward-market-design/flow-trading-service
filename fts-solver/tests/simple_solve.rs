use fts_solver::{AuctionOutcome, DemandCurve, Point, Submission};
use rstest::*;
use rstest_reuse::{self, *};
use std::iter;

mod all_solvers;
use all_solvers::all_solvers;

type HashMap<K, V> = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;

#[fixture]
pub fn bid_data() -> HashMap<usize, Submission<usize, usize>> {
    // Create a submission for a buyer, where we use usize for the id types
    let buyer = {
        let portfolio = iter::once((0, 1.0));
        let curve = DemandCurve {
            domain: (0.0, 1.0),
            group: iter::once((0, 1.0)),
            points: vec![
                Point {
                    quantity: 0.0,
                    price: 10.0,
                },
                Point {
                    quantity: 1.0,
                    price: 5.0,
                },
            ]
            .into_iter(),
        };

        Submission::new(iter::once((0, portfolio)), iter::once(curve)).unwrap()
    };

    // Create a submission for a seller
    let seller = {
        let portfolio = iter::once((0, 1.0));
        let curve = DemandCurve {
            domain: (-1.0, 0.0),
            group: iter::once((0, 1.0)),
            points: vec![
                Point {
                    quantity: -1.0,
                    price: 7.5,
                },
                Point {
                    quantity: 0.0,
                    price: 7.5,
                },
            ]
            .into_iter(),
        };

        Submission::new(iter::once((0, portfolio)), iter::once(curve)).unwrap()
    };

    let mut data = HashMap::default();
    data.insert(0, buyer);
    data.insert(1, seller);

    data
}

#[apply(all_solvers)]
#[rstest]
fn should_success(
    solver: impl fts_solver::Solver,
    bid_data: HashMap<usize, Submission<usize, usize>>,
) {
    let AuctionOutcome {
        mut submissions,
        products,
    } = solver.solve(&bid_data);

    assert_eq!(submissions.len(), 2);
    assert_eq!(products.len(), 1);

    let buyer = submissions
        .swap_remove(&0)
        .unwrap()
        .swap_remove(&0)
        .unwrap();
    let seller = submissions
        .swap_remove(&1)
        .unwrap()
        .swap_remove(&0)
        .unwrap();

    // Check product price and portfolio price, against known good
    assert_eq!((products[0].price * 1000.0).round(), 7500.0);
    assert_eq!((buyer.price * 1000.0).round(), 7500.0);
    assert_eq!((seller.price * 1000.0).round(), 7500.0);

    // Check trades
    assert_eq!((products[0].trade * 1000.0).round(), 500.0);
    assert_eq!((buyer.trade * 1000.0).round(), 500.0);
    assert_eq!((seller.trade * 1000.0).round(), -500.0);
}
