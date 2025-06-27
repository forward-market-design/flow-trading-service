use crate::export::{export_lp, export_mps};
use fts_core::{
    models::{DemandCurve, Map},
    ports::Solver,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::io::Write;
use std::{fmt, hash::Hash, iter};

// First order of business: create some newtype wrappers for the various primitives.

macro_rules! string_wrapper {
    ($struct:ident) => {
        #[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        #[doc = concat!("A newtype wrapper for ", stringify!($struct))]
        pub struct $struct(String);

        impl fmt::Display for $struct {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

string_wrapper!(DemandId);
string_wrapper!(PortfolioId);
string_wrapper!(ProductId);

// Second order of business: create one layer of indirection to allow for "simplified"
// group and portfolio definition in the source file.

fn collection2map<'de, D: Deserializer<'de>, T: Eq + Hash + Deserialize<'de>>(
    data: D,
) -> Result<Map<T>, D::Error> {
    Collection::<T>::deserialize(data).map(Into::into)
}

// This type spells out the 3 ways to define a collection

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Collection<T: Eq + Hash> {
    OneOf(T),
    SumOf(Vec<T>),
    MapOf(Map<T>),
}

impl<T: Eq + Hash> Into<Map<T>> for Collection<T> {
    fn into(self) -> Map<T> {
        match self {
            Collection::OneOf(entry) => iter::once((entry, 1.0)).collect(),
            Collection::SumOf(entries) => entries.into_iter().zip(iter::repeat(1.0)).collect(),
            Collection::MapOf(entries) => entries,
        }
    }
}

/// a representation of a portfolio
#[derive(Debug, Serialize, Deserialize)]
pub struct Portfolio {
    /// the demand curves
    #[serde(deserialize_with = "collection2map")]
    demand_group: Map<DemandId>,
    /// the products
    #[serde(deserialize_with = "collection2map")]
    product_group: Map<ProductId>,
}

/// a representation of an auction
#[derive(Debug, Serialize, Deserialize)]
pub struct Auction {
    /// the demand curves
    pub demand_curves: Map<DemandId, DemandCurve>,
    /// the portfolios
    pub portfolios: Map<PortfolioId, Portfolio>,
}

/// a representation of the solution of an auction
#[derive(Serialize, Deserialize)]
pub struct Outcome<PortfolioOutcome, ProductOutcome> {
    /// the portfolio outcomes
    pub portfolios: Map<PortfolioId, PortfolioOutcome>,
    /// the product outcomes
    pub products: Map<ProductId, ProductOutcome>,
}

impl Auction {
    /// solve the auction
    pub async fn solve<T: Solver<DemandId, PortfolioId, ProductId>>(
        self,
        solver: T,
    ) -> Outcome<T::PortfolioOutcome, T::ProductOutcome> {
        let portfolios = self
            .portfolios
            .into_iter()
            .map(
                |(
                    portfolio_id,
                    Portfolio {
                        demand_group,
                        product_group,
                    },
                )| (portfolio_id, (demand_group, product_group)),
            )
            .collect::<Map<_, _>>();

        let (portfolio_outcomes, product_outcomes) = solver
            .solve(self.demand_curves, portfolios, Default::default())
            .await
            .unwrap();

        Outcome {
            portfolios: portfolio_outcomes,
            products: product_outcomes,
        }
    }

    /// export the auction to LP format
    pub fn export_lp(self, buffer: &mut impl Write) -> Result<(), std::io::Error> {
        let portfolios = self
            .portfolios
            .into_iter()
            .map(
                |(
                    portfolio_id,
                    Portfolio {
                        demand_group,
                        product_group,
                    },
                )| (portfolio_id, (demand_group, product_group)),
            )
            .collect();
        export_lp(self.demand_curves, portfolios, buffer)
    }

    /// export the auction to MPS format
    pub fn export_mps(self, buffer: &mut impl Write) -> Result<(), std::io::Error> {
        let portfolios = self
            .portfolios
            .into_iter()
            .map(
                |(
                    portfolio_id,
                    Portfolio {
                        demand_group,
                        product_group,
                    },
                )| (portfolio_id, (demand_group, product_group)),
            )
            .collect();
        export_mps(self.demand_curves, portfolios, buffer)
    }
}
