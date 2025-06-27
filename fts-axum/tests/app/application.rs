use super::Permissions;
use fts_core::ports::Application;
use fts_solver::clarabel::ClarabelSolver;
use fts_sqlite::{
    Db,
    types::{BidderId, DateTime, DemandId, PortfolioId, ProductId},
};
use headers::{Authorization, authorization::Bearer};

#[derive(Clone)]
pub struct TestApp(pub Db);

impl TestApp {
    /// Extract and verify JWT claims from the authorization header.
    fn permissions(&self, context: &Authorization<Bearer>) -> Option<Permissions> {
        context.0.token().parse().ok()
    }
}

impl Application for TestApp {
    // We will stuff plain-text declarations of the permissions in the token
    type Context = Authorization<Bearer>;

    // Similarly, we stuff the fixed id for the entity in the data
    type DemandData = DemandId;
    type PortfolioData = PortfolioId;
    type ProductData = ProductId;

    type Repository = Db;
    type Solver = ClarabelSolver<DemandId, PortfolioId, ProductId>;

    fn database(&self) -> &Self::Repository {
        &self.0
    }

    fn solver(&self) -> Self::Solver {
        ClarabelSolver::default()
    }

    fn now(&self) -> DateTime {
        time::OffsetDateTime::now_utc().into()
    }

    fn generate_demand_id(&self, data: &Self::DemandData) -> DemandId {
        data.clone()
    }

    fn generate_portfolio_id(&self, data: &Self::PortfolioData) -> PortfolioId {
        data.clone()
    }

    fn generate_product_id(&self, data: &Self::ProductData) -> ProductId {
        data.clone()
    }

    async fn can_create_bid(&self, context: &Self::Context) -> Option<BidderId> {
        self.permissions(context).and_then(|p| {
            if p.can_create_bid {
                p.bidder_id.first().cloned()
            } else {
                None
            }
        })
    }

    async fn can_query_bid(&self, context: &Self::Context) -> Vec<BidderId> {
        self.permissions(context)
            .map(|p| {
                if p.can_query_bid {
                    p.bidder_id
                } else {
                    Vec::new()
                }
            })
            .unwrap_or_default()
    }

    async fn can_update_bid(&self, context: &Self::Context, bidder_id: BidderId) -> bool {
        self.permissions(context)
            .map(|p| {
                if p.can_update_bid {
                    p.bidder_id.iter().any(|id| id == &bidder_id)
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }

    async fn can_read_bid(&self, context: &Self::Context, bidder_id: BidderId) -> bool {
        self.permissions(context)
            .map(|p| {
                if p.can_read_bid {
                    p.bidder_id.iter().any(|id| id == &bidder_id)
                } else {
                    false
                }
            })
            .unwrap_or(false)
    }

    async fn can_view_products(&self, context: &Self::Context) -> bool {
        self.permissions(context)
            .map(|p| p.can_view_products)
            .unwrap_or(false)
    }

    async fn can_manage_products(&self, context: &Self::Context) -> bool {
        self.permissions(context)
            .map(|p| p.can_manage_products)
            .unwrap_or(false)
    }

    async fn can_run_batch(&self, context: &Self::Context) -> bool {
        self.permissions(context)
            .map(|p| p.can_run_batch)
            .unwrap_or(false)
    }
}
