use crate::{DemandCurve, HashMap, Point, Submission, SubmissionError};
use serde::{Deserialize, Serialize};

macro_rules! string_wrapper {
    ($struct:ident) => {
        /// A simple newtype wrapper around a String
        #[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $struct(String);
    };
}

string_wrapper!(ProductId);
string_wrapper!(PortfolioId);
string_wrapper!(BidderId);

#[derive(Serialize, Deserialize)]
struct RawDemandCurve {
    group: HashMap<PortfolioId, f64>,
    points: Vec<Point>,
}

impl RawDemandCurve {
    fn domain(&self) -> (f64, f64) {
        (
            self.points.first().map(|x| x.quantity).unwrap_or_default(),
            self.points.last().map(|x| x.quantity).unwrap_or_default(),
        )
    }
}

#[derive(Serialize, Deserialize)]
struct RawSubmission {
    portfolios: HashMap<PortfolioId, HashMap<ProductId, f64>>,
    demand_curves: Vec<RawDemandCurve>,
}

/// A wrapper for raw auction input, intended for use with serde
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct RawAuction(HashMap<BidderId, RawSubmission>);

impl RawAuction {
    /// Prepare the auction for solution
    pub fn prepare(
        self,
    ) -> Result<HashMap<BidderId, Submission<PortfolioId, ProductId>>, SubmissionError> {
        self.0
            .into_iter()
            .map(
                |(
                    bidder_id,
                    RawSubmission {
                        portfolios,
                        demand_curves,
                    },
                )| {
                    Ok((
                        bidder_id,
                        Submission::new(
                            portfolios
                                .into_iter()
                                .map(|(id, portfolio)| (id, portfolio.into_iter())),
                            demand_curves.into_iter().map(|curve| {
                                let domain = curve.domain();
                                let RawDemandCurve { group, points } = curve;
                                DemandCurve {
                                    domain,
                                    group: group.into_iter(),
                                    points: points.into_iter(),
                                }
                            }),
                        )?,
                    ))
                },
            )
            .collect::<Result<_, _>>()
    }
}
