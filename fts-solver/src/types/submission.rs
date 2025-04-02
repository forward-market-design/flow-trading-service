use super::{Auth, Constant, Cost, Group, PiecewiseLinearCurve, Portfolio};
use crate::{Map, Set};
use std::hash::Hash;

/// A submission is a full expression of an agent's preferences in an auction.
#[derive(Debug)]
pub struct Submission<AuthId: Eq + Hash, ProductId: Eq + Hash> {
    pub(crate) auths_active: Map<AuthId, Auth<ProductId>>,
    pub(crate) auths_inactive: Vec<(AuthId, Portfolio<ProductId>)>,
    pub(crate) costs_curve: Vec<(Group<AuthId>, PiecewiseLinearCurve)>,
    pub(crate) costs_constant: Vec<(Group<AuthId>, Constant)>,
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
    ) -> Self {
        // Step 1: Scan through the costs, bucketing them accordingly and making
        // note of which auths are referenced. (Auths which do not have a cost
        // reference are forced to zero.)

        let mut curves = Vec::new();
        let mut constants = Vec::new();

        // This will allow us to track if auths are referenced by costs or not
        let mut in_use = Set::<AuthId>::default();

        // Partition the costs by type, as well as record which auths are referenced.
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

        // Step 2: Partition the auths into those that can have nonzero trade and those that are necessarily zero.
        // This allows us to omit defunct terms from the optimization but still keep track of them.
        let mut active = Map::default();
        let mut inactive = Vec::new();

        for (auth_id, auth_data) in auths {
            if (auth_data.min_trade < 0.0 || auth_data.max_trade > 0.0)
                && auth_data.portfolio.len() > 0
                && in_use.contains(&auth_id)
            {
                active.insert(auth_id, auth_data);
            } else {
                inactive.push((auth_id, auth_data.portfolio));
            }
        }

        // Step 3. We now "deflate" the groups to eliminate inactive auth references
        for group in curves
            .iter_mut()
            .map(|(group, _)| group)
            .chain(constants.iter_mut().map(|(group, _)| group))
        {
            group.retain(|key, _| active.contains_key(key))
        }

        // 4. Remove any curves, constants with empty groups
        curves.retain(|(group, _)| group.len() > 0);
        constants.retain(|(group, _)| group.len() > 0);

        Submission {
            auths_active: active,
            auths_inactive: inactive,
            costs_curve: curves,
            costs_constant: constants,
        }
    }
}
