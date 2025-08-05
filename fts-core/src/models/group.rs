use std::hash::Hash;

macro_rules! hashmap_newtype {
    ($map:ident, $name:expr) => {
        /// A wrapper around an implementation of a HashMap, with values of f64.
        ///
        /// Predictable and consistent ordering is important to ensure identical solutions
        /// from repeated solves, so we replace the std::collections::HashMap with
        /// indexmap::IndexMap. However, this is an implementation detail, so we wrap it
        /// in a newtype, allowing us to replace it at a future date without breaking
        /// semver. This unfortunately leads to additional boiler-plate, but at least it
        /// is not particularly complicated.
        #[derive(Debug, Clone, PartialEq)]
        #[cfg_attr(feature = "schemars", derive(schemars::JsonSchema), schemars(rename = $name))]
        #[cfg_attr(
            feature = "serde",
            derive(serde::Serialize, serde::Deserialize),
            serde(
                from = "Collection::<K>",
                into = "Collection::<K>",
                bound(serialize = "K: serde::Serialize + Clone")
            )
        )]
        pub struct $map<K: Eq + Hash>(indexmap::IndexMap<K, f64, rustc_hash::FxBuildHasher>);

        impl<K: Eq + Hash> Default for $map<K> {
            fn default() -> Self {
                Self(indexmap::IndexMap::default())
            }
        }

        impl<K: Eq + Hash> std::ops::Deref for $map<K> {
            type Target = indexmap::IndexMap<K, f64, rustc_hash::FxBuildHasher>;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<K: Eq + Hash> std::ops::DerefMut for $map<K> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl<K: Eq + Hash> IntoIterator for $map<K> {
            type Item = (K, f64);
            type IntoIter = indexmap::map::IntoIter<K, f64>;

            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }

        impl<K: Eq + Hash> FromIterator<(K, f64)> for $map<K> {
            fn from_iter<I: IntoIterator<Item = (K, f64)>>(iter: I) -> Self {
                Self(indexmap::IndexMap::from_iter(iter))
            }
        }

        impl<K: Eq + Hash> From<Collection<K>> for $map<K> {
            fn from(value: Collection<K>) -> Self {
                match value {
                    Collection::Empty => Default::default(),
                    Collection::OneOf(entry) => std::iter::once((entry, 1.0)).collect(),
                    Collection::SumOf(entries) => {
                        entries.into_iter().zip(std::iter::repeat(1.0)).collect()
                    }
                    Collection::MapOf(entries) => Self(entries),
                }
            }
        }

        impl<K: Eq + Hash + Clone> Into<Collection<K>> for $map<K> {
            fn into(self) -> Collection<K> {
                if self.0.len() > 0 {
                    Collection::MapOf(self.0)
                } else {
                    Collection::Empty
                }
            }
        }
    };
}

// For now, we implement demand- and product-groups the same way, though this
// allows us to change the implementations separately. (For example, maybe we
// switch the DemandGroup implementation to be optimal for assumed-small hash
// tables.)

hashmap_newtype!(DemandGroup, "DemandGroup");
hashmap_newtype!(PortfolioGroup, "PortfolioGroup");
hashmap_newtype!(ProductGroup, "ProductGroup");

// This type spells out the 3 ways to define a collection

#[derive(Debug)]
#[cfg_attr(
    feature = "schemars",
    derive(schemars::JsonSchema),
    schemars(rename = "{T}Group", untagged)
)]
enum Collection<T: Eq + Hash> {
    Empty,
    OneOf(T),
    SumOf(Vec<T>),
    MapOf(indexmap::IndexMap<T, f64, rustc_hash::FxBuildHasher>),
}

#[cfg(feature = "serde")]
impl<'de, T: Eq + Hash + serde::Deserialize<'de>> serde::Deserialize<'de> for Collection<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde_untagged::UntaggedEnumVisitor::new()
            .unit(|| Ok(Self::Empty))
            .string(|one| {
                T::deserialize(serde::de::value::StrDeserializer::new(one)).map(Self::OneOf)
            })
            .seq(|sum| sum.deserialize().map(Self::SumOf))
            .map(|map| map.deserialize().map(Self::MapOf))
            .deserialize(deserializer)
    }
}

#[cfg(feature = "serde")]
impl<T: Eq + Hash + serde::Serialize> serde::Serialize for Collection<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Empty => serializer.serialize_none(),
            Self::OneOf(one) => one.serialize(serializer),
            Self::SumOf(sum) => sum.serialize(serializer),
            Self::MapOf(map) => map.serialize(serializer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_scalar() {
        let result: Result<Collection<String>, _> = serde_json::from_str(r#""apples""#);
        match result {
            Ok(Collection::OneOf(_)) => (),
            Ok(_) => {
                panic!("scalar parsed incorrectly");
            }
            Err(_) => {
                panic!("could not parse collection");
            }
        };
    }

    #[test]
    fn test_from_vector() {
        let result: Result<Collection<String>, _> =
            serde_json::from_str(r#"["apples", "bananas"]"#);
        match result {
            Ok(Collection::SumOf(_)) => (),
            Ok(_) => {
                panic!("scalar parsed incorrectly");
            }
            Err(_) => {
                panic!("could not parse collection");
            }
        };
    }

    #[test]
    fn test_from_map() {
        let result: Result<Collection<String>, _> =
            serde_json::from_str(r#"{"apples": 1, "bananas": 1}"#);
        match result {
            Ok(Collection::MapOf(_)) => (),
            Ok(_) => {
                panic!("scalar parsed incorrectly");
            }
            Err(_) => {
                panic!("could not parse collection");
            }
        };
    }

    #[test]
    fn test_to_scalar() {
        match serde_json::to_value(Collection::OneOf("apples".to_owned())) {
            Ok(serde_json::Value::String(value)) => assert_eq!(value, "apples"),
            Ok(_) => panic!("scalar serialized incorrectly"),
            Err(_) => panic!("could not serialize collection"),
        }
    }

    #[test]
    fn test_to_vector() {
        match serde_json::to_value(Collection::SumOf(vec![
            "apples".to_owned(),
            "bananas".to_owned(),
        ])) {
            Ok(serde_json::Value::Array(value)) => assert_eq!(value.len(), 2),
            Ok(_) => panic!("vector serialized incorrectly"),
            Err(_) => panic!("could not serialize collection"),
        }
    }

    #[test]
    fn test_to_map() {
        match serde_json::to_value(Collection::MapOf(
            ["apples", "bananas"]
                .into_iter()
                .map(|fruit| (fruit.to_owned(), 1.0))
                .collect(),
        )) {
            Ok(serde_json::Value::Object(value)) => assert_eq!(value.len(), 2),
            Ok(_) => panic!("map serialized incorrectly"),
            Err(_) => panic!("could not serialize collection"),
        }
    }
}
