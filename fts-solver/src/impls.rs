use crate::{HashSet, Submission};
use std::hash::Hash;

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
    PortfolioId: Eq + Hash + Clone + Ord,
    ProductId: Eq + Hash + Clone + Ord,
>(
    auction: &T,
) -> (HashSet<ProductId>, usize)
where
    for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<PortfolioId, ProductId>)>,
{
    // In order to setup the the optimization program, we need to define
    // up front the full space of products, as well as assign a canonical
    // enumerative index to each of them. Due to the structure of the matrix
    // we also need to know the total number of costs.

    let mut products = HashSet::default();
    let mut ncosts = 0;

    for (_, submission) in auction.into_iter() {
        products.extend(
            submission
                .portfolios
                .values()
                .flat_map(|portfolio| portfolio.keys().map(Clone::clone)),
        );

        ncosts += submission.demand_curves.len();
    }

    // Provide a canonical ordering to the product ids
    products.sort_unstable();

    (products, ncosts)
}
