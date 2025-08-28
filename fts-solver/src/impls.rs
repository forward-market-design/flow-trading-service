use crate::HashSet;
use fts_core::{
    models::{Basis, DemandCurve, Map, Weights},
    ports::Outcome,
};
use std::hash::Hash;

/// Implementation using the Clarabel interior point solver
#[cfg(feature = "clarabel")]
pub mod clarabel;

/// Implementation using the OSQP operator splitting solver
#[cfg(feature = "osqp")]
pub mod osqp;

// A helper method that prepares an auction by canonicalizing and sorting elements
// in a manner that facilitates CSC matrix construction
pub(crate) fn prepare<
    DemandId: Clone + Eq + Hash,
    PortfolioId: Clone + Eq + Hash,
    ProductId: Clone + Eq + Hash + Ord,
>(
    mut demand_curves: Map<DemandId, DemandCurve>,
    mut portfolios: Map<PortfolioId, (Weights<DemandId>, Basis<ProductId>)>,
) -> (
    Map<DemandId, DemandCurve>,
    Map<PortfolioId, (Weights<DemandId>, Basis<ProductId>)>,
    Map<PortfolioId, Outcome<()>>,
    Map<ProductId, Outcome<()>>,
) {
    // Construct the "basis" of products.
    let mut product_outcomes = Map::<ProductId, Outcome<()>>::default();

    // Initialize the portfolio outcomes.
    let mut portfolio_outcomes = Map::<PortfolioId, Outcome<()>>::default();
    portfolio_outcomes.reserve_exact(portfolios.len());

    // Canonicalize the input so we can avoid some awkward error handling later
    let mut demands_in_use = HashSet::default();

    for (portfolio_id, (demand, basis)) in portfolios.iter_mut() {
        // We keep only demands that are known to exist (and have non-zero weight)
        demand.retain(|demand_id, weight| *weight != 0.0 && demand_curves.contains_key(demand_id));
        // We ensure the associated demands are sorted by the ordering defined by the demand lookup
        demand.sort_unstable_by(|a, _, b, _| {
            demand_curves
                .get_index_of(a)
                .cmp(&demand_curves.get_index_of(b))
        });
        // And we record which demands are in use
        demands_in_use.extend(demand.keys());

        // We keep only non-zero portfolio entries
        basis.retain(|_, &mut weight| weight != 0.0);
        basis.sort_unstable_keys();

        // Ensure we have slots to put required outcomes
        portfolio_outcomes.entry(portfolio_id.clone()).or_default();
        for product_id in basis.keys() {
            product_outcomes.entry(product_id.clone()).or_default();
        }
    }

    // Assign a canonical ordering to products
    product_outcomes.sort_unstable_keys();

    // Remove any unused demand curves (retaining preserves the relative ordering of preserved elements, so our earlier sorting is good)
    demand_curves.retain(|demand_id, _| demands_in_use.contains(demand_id));

    (
        demand_curves,
        portfolios,
        portfolio_outcomes,
        product_outcomes,
    )
}

// A helper method that appropriately populates the outcomes given solver output
pub(crate) fn finalize<
    'a,
    'b,
    DemandId: Clone + Eq + Hash,
    PortfolioId: Clone + Eq + Hash,
    ProductId: Clone + Eq + Hash + Ord,
>(
    mut primal: impl Iterator<Item = &'a f64>,
    dual: impl Iterator<Item = &'b f64>,
    portfolios: &Map<PortfolioId, (Weights<DemandId>, Basis<ProductId>)>,
    portfolio_outcomes: &mut Map<PortfolioId, Outcome<()>>,
    product_outcomes: &mut Map<ProductId, Outcome<()>>,
) {
    // 1. Set the product prices, leaving their trade at 0.
    for (product_outcome, &price) in product_outcomes.values_mut().zip(dual) {
        product_outcome.price = if price.is_finite() { Some(price) } else { None };
    }
    // 2. For each portfolio...
    for (portfolio_id, (demand, basis)) in portfolios.iter() {
        let trade = if demand.len() == 0 || basis.len() == 0 {
            0.0
        } else {
            // SAFETY: this should never panic, since we always add a decision variable for the above condition
            *primal.next().unwrap()
        };

        // SAFETY: this unwrap() is guaranteed by the logic in prepare()
        let portfolio_outcome = portfolio_outcomes.get_mut(portfolio_id).unwrap();

        // Copy the determined rate...
        portfolio_outcome.trade = trade;

        // ... and simultaneously construct the effective price and update the product trade volume
        if trade != 0.0 && basis.len() > 0 {
            let mut price = 0.0;
            for (product_id, weight) in basis.iter() {
                let product_outcome = product_outcomes.get_mut(product_id).unwrap();
                price += product_outcome
                    .price
                    .expect("a traded portfolio somehow did not produce an underlying price")
                    * weight;
                product_outcome.trade += (weight * trade).abs();
            }
            portfolio_outcome.price = Some(price);
        }
    }

    // We have double-counted the trade for each product, so we halve it
    for product_outcome in product_outcomes.values_mut() {
        product_outcome.trade *= 0.5;
    }
}
