use super::{Auth, Constant, Cost, Group, PiecewiseLinearCurve};
use crate::{Map, Set};
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// A submission is a full expression of an agent's preferences in an auction.
#[derive(Debug, Serialize, Deserialize)]
pub struct Submission<AuthId: Eq + Hash, ProductId: Eq + Hash> {
    pub(crate) auths: Map<AuthId, Auth<ProductId>>,
    pub(crate) cost_curves: Vec<(Group<AuthId>, PiecewiseLinearCurve)>,
    pub(crate) cost_constants: Vec<(Group<AuthId>, Constant)>,
}

impl<AuthId: Eq + Hash + Clone, ProductId: Eq + Hash + Clone> Submission<AuthId, ProductId> {
    /// Creates a new submission from the provided auths and costs.
    /// All data is assumed to be valid.
    pub fn new<
        A: IntoIterator<Item = (AuthId, Auth<ProductId>)>,
        B: IntoIterator<Item = (Group<AuthId>, Cost)>,
    >(
        auths: A,
        costs: B,
    ) -> Option<Self> {
        // We filter the auths down to only those that allow for non-zero trade.
        // We will later apply an additional filter that further removes any auths
        // that are not otherwise referenced by a cost.
        let mut auths = auths
            .into_iter()
            .filter(
                |(
                    _,
                    Auth {
                        min_trade,
                        max_trade,
                        portfolio,
                    },
                )| {
                    (*min_trade < 0.0 || *max_trade > 0.0) && portfolio.len() > 0
                },
            )
            .collect::<Map<AuthId, Auth<ProductId>>>();

        // We also will partition the submission's costs into the various
        // supported types of cost specification.
        let mut curves = Vec::new();
        let mut constants = Vec::new();

        // This will allow us to track if auths are referenced by costs or not
        let mut in_use = Set::<AuthId>::default();

        // 2. Partition the costs by type, as well as record which auths are referenced.
        for (group, cost) in costs.into_iter() {
            // Record any auths in use (the keys() method of group will not return zero-valued entries)
            in_use.extend(group.keys().map(|x| x.clone()));
            // Then we just partition into the relevant bins. If we ever support
            // additional cost curve types, we can just add another field to the Submission
            // and handle the partitioning here.
            match cost {
                Cost::PiecewiseLinearCurve(curve) => {
                    curves.push((group, curve));
                }
                Cost::Constant(constant) => {
                    constants.push((group, constant));
                }
            }
        }

        // 2. Remove any authorizations not otherwise referenced.
        // THIS IS VERY IMPORTANT, as otherwise we may end up creating an unconstrained decision variable
        auths.retain(|id, _| in_use.contains(id));

        // 3. We now "deflate" the groups to eliminate stale auth references
        for group in curves
            .iter_mut()
            .map(|(group, _)| group)
            .chain(constants.iter_mut().map(|(group, _)| group))
        {
            group.retain(|key, _| auths.get(key).is_some())
        }

        // 4. Remove any curves, constants with empty groups
        curves.retain(|(group, _)| group.len() > 0);
        constants.retain(|(group, _)| group.len() > 0);

        if curves.len() > 0 || constants.len() > 0 {
            Some(Submission {
                auths,
                cost_curves: curves,
                cost_constants: constants,
            })
        } else {
            None
        }
    }
}
