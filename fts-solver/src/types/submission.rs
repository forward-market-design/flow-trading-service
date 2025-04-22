use super::{DemandCurve, Segment, demand::Point, disaggregate};
use crate::{HashMap, HashSet};
use std::hash::Hash;
use thiserror::Error;

/// The fundamental input to a `Solver` implementation, containing an
/// independent collection of portfolios and demand curves.
#[derive(Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(bound = "
        PortfolioId: Clone + Hash + Eq + serde::Serialize + serde::de::DeserializeOwned,
        ProductId: Hash + Eq + Ord + serde::Serialize + serde::de::DeserializeOwned
    ")
)]
pub struct Submission<PortfolioId, ProductId> {
    /// The portfolios that are defined by the submission.
    pub portfolios: HashMap<PortfolioId, HashMap<ProductId, f64>>,

    /// The demand curves in the submission
    pub demand_curves: Vec<(HashMap<PortfolioId, f64>, Vec<Segment>)>,
}

impl<PortfolioId: Clone + Hash + Eq, ProductId: Hash + Eq + Ord>
    Submission<PortfolioId, ProductId>
{
    /// Construct a canonicalized submission from the inputs
    pub fn new<T, U, V>(
        portfolios: impl IntoIterator<Item = (PortfolioId, T)>,
        curves: impl IntoIterator<Item = DemandCurve<PortfolioId, U, V>>,
    ) -> Result<Submission<PortfolioId, ProductId>, SubmissionError>
    where
        T: IntoIterator<Item = (ProductId, f64)>,
        T::IntoIter: ExactSizeIterator,
        U: IntoIterator<Item = (PortfolioId, f64)>,
        V: IntoIterator<Item = Point>,
    {
        // Step 1: Canonicalize the portfolios
        let portfolios = portfolios
            .into_iter()
            .map(|(id, weights)| {
                let weights = weights.into_iter();
                let mut portfolio = HashMap::<ProductId, f64>::with_capacity_and_hasher(
                    weights.len(),
                    Default::default(),
                );

                // If product entries are repeated, sum them
                for (product, weight) in weights {
                    *portfolio.entry(product).or_default() += weight;
                }

                // For maximal sparsity, keep only non-zero terms
                portfolio.retain(|_, v| *v != 0.0);

                // Sort the portfolio by product id, so that subsequent matrix
                // construction can rely on this invariant.
                // (Note: we can enforce this invariant elsewhere, if we want to remove this for performance.)
                portfolio.sort_unstable_keys();

                (id, portfolio)
            })
            .collect::<HashMap<_, _>>();

        // Each portfolio creates a decision variable, but if the variable is
        // not referenced by a cost, it becomes unconstrained. We *define* the
        // behavior in this case to force unconstrained portfolios to zero.
        // To start, we assert every portfolio is unused, then remove entries
        // as we iterate through the demand curves.
        let mut unused: HashSet<&PortfolioId> = portfolios.keys().collect();

        // Step 2: Canonicalize the demand curves
        let mut curves = curves
            .into_iter()
            .filter_map(
                |DemandCurve {
                     domain: (min, max),
                     group,
                     points,
                 }| {
                    // Collect the weights into a sparse vector
                    let group = {
                        // Aggreggate any repeated entries
                        let mut group2 = HashMap::default();
                        for (id, weight) in group {
                            *group2.entry(id).or_default() += weight;
                        }

                        // Maximize the sparsity of the vector
                        group2.retain(|id, weight| {
                            // We only keep this pair if the associated portfolio
                            // (1) exists and (2) has at least one product to trade
                            // and the weight is nonzero.
                            let portfolio_size = portfolios
                                .get(id)
                                .map(|portfolio| portfolio.len())
                                .unwrap_or(0);

                            if portfolio_size != 0 && *weight != 0.0 {
                                // If we keep the pair, make sure we accredit `unused`
                                unused.swap_remove(id);
                                true
                            } else {
                                false
                            }
                        });

                        // If the group is empty, just ignore this curve entirely
                        if group2.len() == 0 {
                            return None;
                        }

                        group2
                    };

                    // Decompose the curve into its constituent segments
                    let segments = disaggregate(points.into_iter(), min, max)
                        .map(|iter| iter.collect::<Result<Vec<_>, _>>());

                    match segments {
                        Some(Ok(segments)) => Some(Ok((group, segments))),
                        Some(Err(error)) => Some(Err(SubmissionError::InvalidDemandCurve(error))),
                        None => Some(Err(SubmissionError::InvalidDomain(min, max))),
                    }
                },
            )
            .collect::<Result<Vec<_>, _>>()?;

        // Now, anything left in `unused` was not referenced by any demand curve.
        // Accordingly, we inject demand curves that force them to zero.
        for portfolio in unused {
            let mut group = HashMap::with_capacity_and_hasher(1, Default::default());
            group.insert(portfolio.clone(), 1.0);
            curves.push((group, Vec::new()))
        }

        // Note: there are opportunities for additional gains here. For example,
        // subsetting the demand curves (group, segments) on segments.len() == 0
        // gives us a presolve step where we can solve Ax = 0 (rows of A are the
        // groups) and determine which x are necessarily zero, which can be
        // propagated upwards to further sparsify the groups as well as remove the
        // associated portfolio.
        //
        // Since the underlying QP solver does its own linear algebra, we do not
        // exploit this presolve step in this constructor. However, we intend to
        // make a Submission::presolve(&mut self) function that does this and other
        // such calculations.
        Ok(Self {
            portfolios,
            demand_curves: curves,
        })
    }

    // /// Computes which portfolios are necessarily zero and removes them from the submission
    // pub fn presolve(&mut self) -> HashSet<PortfolioId> {
    //     unimplemented!("TODO");
    // }
}

/// The error type for when preparing a submission fails
#[derive(Error, Debug)]
pub enum SubmissionError {
    /// When a demand curve cannot be disaggregated
    #[error("invalid demand curve")]
    InvalidDemandCurve(Segment),

    /// When an invalid truncation is specified for a demand curve
    #[error("invalid domain")]
    InvalidDomain(f64, f64),
}

#[cfg(test)]
mod tests {
    //use super::*;

    // TODO
}
