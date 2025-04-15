use crate::{Auction, DemandCurve, HashMap, Point, Submission, SubmissionError};
use serde::{Deserialize, Deserializer, Serialize};
use std::{hash::Hash, iter, ops::Deref};

// First order of business: create some newtype wrappers for the various primitives.

macro_rules! string_wrapper {
    ($struct:ident) => {
        #[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        #[doc = concat!("A newtype wrapper for ", stringify!($struct))]
        pub struct $struct(String);
    };
}

string_wrapper!(ProductId);
string_wrapper!(PortfolioId);
string_wrapper!(BidderId);

// Second order of business: create one layer of indirection to allow for "simplified"
// group and portfolio definition in the source file.

macro_rules! map_wrapper {
    ($struct:ident, $key:ty) => {
        /// A newtype wrapper for a Portfolio
        #[derive(Debug, Serialize, Deserialize)]
        #[doc = concat!("A newtype wrapper for ", stringify!($struct))]
        pub struct $struct(#[serde(deserialize_with = "collection2map")] HashMap<$key, f64>);

        impl Deref for $struct {
            type Target = HashMap<$key, f64>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl IntoIterator for $struct {
            type Item = ($key, f64);
            type IntoIter = <HashMap<$key, f64> as IntoIterator>::IntoIter;

            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }
    };
}

map_wrapper!(Portfolio, ProductId);
map_wrapper!(Group, PortfolioId);

fn collection2map<'de, D: Deserializer<'de>, T: Eq + Hash + Deserialize<'de>>(
    data: D,
) -> Result<HashMap<T, f64>, D::Error> {
    Collection::<T>::deserialize(data).map(Into::into)
}

// This type spells out the 3 ways to define a collection

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Collection<T: Eq + Hash> {
    OneOf(T),
    SumOf(Vec<T>),
    MapOf(HashMap<T, f64>),
}

impl<T: Eq + Hash> Into<HashMap<T, f64>> for Collection<T> {
    fn into(self) -> HashMap<T, f64> {
        match self {
            Collection::OneOf(entry) => iter::once((entry, 1.0)).collect(),
            Collection::SumOf(entries) => entries.into_iter().zip(iter::repeat(1.0)).collect(),
            Collection::MapOf(entries) => entries,
        }
    }
}

// Now we just define the types necessary to roll up to a full submission

#[derive(Serialize, Deserialize)]
struct DemandCurveDto {
    // If omitted, the domain will be inferred from the points.
    // If specified, the curve will be interpolated or extrapolated accordingly.
    // If either bound is None, will use the appropriately signed infinity.
    domain: Option<(Option<f64>, Option<f64>)>,
    group: Group,
    points: Vec<Point>,
}

impl Into<DemandCurve<PortfolioId, Group, Vec<Point>>> for DemandCurveDto {
    fn into(self) -> DemandCurve<PortfolioId, Group, Vec<Point>> {
        let domain = self
            .domain
            .map(|(min, max)| {
                (
                    min.unwrap_or(f64::NEG_INFINITY),
                    max.unwrap_or(f64::INFINITY),
                )
            })
            .unwrap_or_else(|| {
                (
                    self.points.first().map(|x| x.quantity).unwrap_or_default(),
                    self.points.last().map(|x| x.quantity).unwrap_or_default(),
                )
            });

        DemandCurve {
            domain,
            group: self.group,
            points: self.points,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SubmissionDto {
    portfolios: HashMap<PortfolioId, Portfolio>,
    demand_curves: Vec<DemandCurveDto>,
}

/// A wrapper for raw auction input, intended for use with serde
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct AuctionDto(HashMap<BidderId, SubmissionDto>);

impl TryInto<Auction<BidderId, PortfolioId, ProductId>> for AuctionDto {
    type Error = SubmissionError;

    fn try_into(self) -> Result<Auction<BidderId, PortfolioId, ProductId>, Self::Error> {
        self.0
            .into_iter()
            .map(
                |(
                    bidder_id,
                    SubmissionDto {
                        portfolios,
                        demand_curves,
                    },
                )| {
                    Ok((
                        bidder_id,
                        Submission::new(portfolios, demand_curves.into_iter().map(Into::into))?,
                    ))
                },
            )
            .collect::<Result<_, _>>()
    }
}
