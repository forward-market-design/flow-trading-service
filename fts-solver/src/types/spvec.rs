// We have a few types (Portfolio, Group) that act like HashMaps, but we want
// to reserve the right to swap out std::collection::HashMap with something
// more performant. We also don't want entries with a value of 0 in the final
// representation, so we handle that book-keeping here as well.

/// Macro that creates a sparse vector type with a specific name.
/// The resulting type efficiently stores non-zero weighted elements,
/// filters zero values, and maintains a sorted representation.
macro_rules! spvec {
    ($struct:ident) => {
        /// A sparse collection of weighted elements where zero-valued entries are excluded.
        /// Used for efficiently representing portfolios, groups, and other weighted collections.
        #[derive(Debug)]
        #[repr(transparent)]
        pub struct $struct<T: Eq + ::std::hash::Hash>(
            ::indexmap::IndexMap<T, f64, ::fxhash::FxBuildHasher>,
        );

        impl<T: Eq + ::std::hash::Hash> $struct<T> {
            /// A consuming iterator for the key-value pairs
            /// Returns only non-zero weighted pairs.
            pub fn into_iter(self) -> impl Iterator<Item = (T, f64)> {
                self.0.into_iter().filter_map(|(key, value)| {
                    if value != 0.0 {
                        Some((key, value))
                    } else {
                        None
                    }
                })
            }

            /// A by-reference iterator for the key-value pairs
            /// Returns only non-zero weighted pairs.
            pub fn iter(&self) -> impl Iterator<Item = (&T, &f64)> {
                self.0.iter().filter_map(|(key, value)| {
                    if *value != 0.0 {
                        Some((key, value))
                    } else {
                        None
                    }
                })
            }

            /// Iterate over the keys with non-zero values
            pub fn keys(&self) -> impl Iterator<Item = &T> {
                self.iter().map(|(key, _)| key)
            }

            /// Remove keys based on some criteria
            /// This allows filtering the sparse vector with a custom function.
            pub fn retain<F: Fn(&T, &f64) -> bool>(&mut self, f: F) {
                self.0.retain(|key, value| f(key, value))
            }

            /// The number of key-value pairs. (After a call to .simplify(), equivalent to an L0 norm)
            pub fn len(&self) -> usize {
                self.0.len()
            }

            /// Get the value associated to the key, or None if zero or missing
            pub fn get<Q: ?Sized + ::std::hash::Hash + ::indexmap::Equivalent<T>>(
                &self,
                key: &Q,
            ) -> Option<f64> {
                self.0
                    .get(key)
                    .map(|&x| x)
                    .and_then(|value| if value == 0.0 { None } else { Some(value) })
            }

            /// Is this an economically valid portfolio?
            /// Checks that all weights are finite values.
            pub fn validate(&self) -> bool {
                self.0.values().all(|w| w.is_finite())
            }
        }

        impl<T: Eq + ::std::hash::Hash> Default for $struct<T> {
            fn default() -> Self {
                Self(::indexmap::IndexMap::<T, f64, ::fxhash::FxBuildHasher>::default())
            }
        }

        impl<T: Eq + ::std::hash::Hash + Ord> FromIterator<(T, f64)> for $struct<T> {
            fn from_iter<U: IntoIterator<Item = (T, f64)>>(iter: U) -> Self {
                let mut dict = Self::default();
                for (key, value) in iter.into_iter() {
                    *dict.0.entry(key).or_default() += value;
                }
                // TODO: explore if moving the check to the inside is better
                dict.0.retain(|_, value| *value != 0.0);

                // TODO: this is often unnecessary, since the inputs tend to arrive already sorted.
                // Maybe we should move some of the iteration options to a Sorted newtype, and provide
                // an unsafe method to bypass the intermediate construction.
                // It is important, though, that by the time we get to the solver implementations, we are
                // sorted, so that we do not construct the sparse matrices out-of-order.
                dict.0.sort_unstable_keys();

                dict
            }
        }
    };
}

pub(crate) use spvec;
