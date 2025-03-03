use std::iter;

use rstest::*;
use rstest_reuse::{self, *};

use fts_solver::{
    AuctionOutcome, Auth, Cost, Group, PiecewiseLinearCurve, Point, Portfolio, Submission,
};

mod all_solvers;
use all_solvers::all_solvers;

#[fixture]
pub fn bid_data() -> Vec<Submission<usize, usize>> {
    // Create a submission for a buyer, where we use usize for the id types
    let buyer = {
        // Create a portfolio with a single product (id=0) with weight 1.0
        let portfolio: Portfolio<usize> = iter::once((0, 1.0)).collect();

        // Assign this portfolio an id=0 and authorize it for buy-only trade
        let auth = vec![(
            0,
            Auth {
                min_trade: 0.0,
                max_trade: 1.0,
                portfolio,
            },
        )];

        // Create a bid with a group weight of portfolio(id=0) = 1.0
        let group: Group<usize> = iter::once((0, 1.0)).collect();
        let curve = PiecewiseLinearCurve {
            points: vec![
                Point {
                    quantity: 0.0,
                    price: 10.0,
                },
                Point {
                    quantity: 1.0,
                    price: 5.0,
                },
            ],
        };

        Submission::new(auth, vec![(group, Cost::PiecewiseLinearCurve(curve))])
    };

    let seller = {
        // Create a portfolio with a single product (id=0) with weight 1.0
        let portfolio: Portfolio<usize> = iter::once((0, 1.0)).collect();

        // Assign this portfolio an id=1 and authorize it for sell-only trade
        // (Note that the auth id is different from the buyer's)
        let auth = vec![(
            1,
            Auth {
                min_trade: -1.0,
                max_trade: 0.0,
                portfolio,
            },
        )];

        // Create a bid with a group weight of portfolio(id=0) = 1.0
        let group: Group<usize> = iter::once((1, 1.0)).collect();
        let curve = PiecewiseLinearCurve {
            points: vec![
                Point {
                    quantity: -1.0,
                    price: 7.5,
                },
                Point {
                    quantity: 0.0,
                    price: 7.5,
                },
            ],
        };

        Submission::new(auth, vec![(group, Cost::PiecewiseLinearCurve(curve))])
    };

    vec![buyer.unwrap(), seller.unwrap()]
}

#[apply(all_solvers)]
#[rstest]
fn should_success(solver: impl fts_solver::Solver, bid_data: Vec<Submission<usize, usize>>) {
    let AuctionOutcome { auths, products } = solver.solve(&bid_data);

    assert_eq!(auths.len(), 2);
    assert_eq!(products.len(), 1);

    // Check product price and portfolio price, against known good
    assert_eq!((products[0].price * 1000.0).round(), 7500.0);
    assert_eq!((auths[0].price * 1000.0).round(), 7500.0);
    assert_eq!((auths[1].price * 1000.0).round(), 7500.0);

    // Check trades
    assert_eq!((products[0].volume * 1000.0).round(), 500.0);
    assert_eq!((auths[0].trade * 1000.0).round(), 500.0);
    assert_eq!((auths[1].trade * 1000.0).round(), -500.0);
}
