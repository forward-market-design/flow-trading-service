use std::hash::Hash;

use crate::{Map, Set, Submission};

// #[cfg(feature = "admm")]
// pub mod admm;

/// Implementation using the Clarabel interior point solver
#[cfg(feature = "clarabel")]
pub mod clarabel;

/// Implementation using the OSQP operator splitting solver
#[cfg(feature = "osqp")]
pub mod osqp;

// An internal, shared method for doing the first pass of the auction data.
// (This data is roughly everything you need for input to the actual
// construction of the flow trading QP, e.g. matrix sizes.)
pub(crate) fn prepare<
    T,
    BidderId: Eq + Hash + Clone + Ord,
    AuthId: Eq + Hash + Clone + Ord,
    ProductId: Eq + Hash + Clone + Ord,
>(
    auction: &T,
) -> (Map<AuthId, BidderId>, Map<ProductId, usize>, usize)
where
    for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<AuthId, ProductId>)>,
{
    // In order to setup the the optimization program, we need to define
    // up front the full space of products, as well as assign a canonical
    // enumerative index to each of them. While we're doing this, we also
    // construct a reverse map of auth id to bidder id, for reporting the
    // solution.

    let mut auths = Map::default();
    let mut products = Set::default();
    let mut ncosts = 0;

    for (bidder, submission) in auction.into_iter() {
        for (auth_id, auth_data) in submission.auths.iter() {
            auths.insert(auth_id.clone(), bidder.clone());
            products.extend(auth_data.portfolio.keys());
        }
        ncosts += submission.cost_curves.len() + submission.cost_constants.len();
    }

    // Provide a canonical ordering to the product ids
    products.sort_unstable();

    // Build the index lookup
    let products = products
        .into_iter()
        .enumerate()
        .map(|(a, b)| (b.clone(), a))
        .collect();

    (auths, products, ncosts)
}
