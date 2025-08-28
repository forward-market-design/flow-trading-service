use crate::export::{export_lp, export_mps};
use fts_core::{
    models::{Basis, DemandCurve, Map, Weights},
    ports::Solver,
};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{fmt, hash::Hash};

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

/// a representation of a portfolio
#[derive(Debug, Serialize, Deserialize)]
pub struct Portfolio {
    /// the demand curves
    demand: Weights<DemandId>,
    /// the products
    basis: Basis<ProductId>,
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
    pub portfolios: Map<PortfolioId, fts_core::ports::Outcome<PortfolioOutcome>>,
    /// the product outcomes
    pub products: Map<ProductId, fts_core::ports::Outcome<ProductOutcome>>,
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
            .map(|(portfolio_id, Portfolio { demand, basis })| (portfolio_id, (demand, basis)))
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
            .map(|(portfolio_id, Portfolio { demand, basis })| (portfolio_id, (demand, basis)))
            .collect();
        export_lp(self.demand_curves, portfolios, buffer)
    }

    /// export the auction to MPS format
    pub fn export_mps(self, buffer: &mut impl Write) -> Result<(), std::io::Error> {
        let portfolios = self
            .portfolios
            .into_iter()
            .map(|(portfolio_id, Portfolio { demand, basis })| (portfolio_id, (demand, basis)))
            .collect();
        export_mps(self.demand_curves, portfolios, buffer)
    }
}
