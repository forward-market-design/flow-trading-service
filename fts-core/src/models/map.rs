use std::hash::Hash;

/// A wrapper around an implementation of a HashMap, defaulting to values of f64.
///
/// Predictable and consistent ordering is important to ensure identical solutions
/// from repeated solves, so we replace the std::collections::HashMap with
/// indexmap::IndexMap. However, this is an implementation detail, so we wrap it
/// in a newtype, allowing us to replace it at a future date without breaking
/// semver. This unfortunately leads to additional boiler-plate, but at least it
/// is not particularly complicated.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct Map<K: Eq + Hash, V = f64>(indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>);

impl<K: Eq + Hash, V> Default for Map<K, V> {
    fn default() -> Self {
        Self(indexmap::IndexMap::default())
    }
}

impl<K: Eq + Hash, V> std::ops::Deref for Map<K, V> {
    type Target = indexmap::IndexMap<K, V, rustc_hash::FxBuildHasher>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: Eq + Hash, V> std::ops::DerefMut for Map<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K: Eq + Hash, V> IntoIterator for Map<K, V> {
    type Item = (K, V);
    type IntoIter = indexmap::map::IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<K: Eq + Hash, V> FromIterator<(K, V)> for Map<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self(indexmap::IndexMap::from_iter(iter))
    }
}
