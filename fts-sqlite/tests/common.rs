use fts_core::ports::Application;
use fts_solver::clarabel::ClarabelSolver;
use fts_sqlite::{
    Db,
    types::{BidderId, DateTime, DemandId, PortfolioId, ProductId},
};

pub struct TestApp(pub Db);

impl Application for TestApp {
    type Context = ();
    type DemandData = ();
    type PortfolioData = ();
    type ProductData = ();
    type Repository = Db;
    type Solver = ClarabelSolver<DemandId, PortfolioId, ProductId>;

    fn database(&self) -> &Self::Repository {
        &self.0
    }

    fn now(&self) -> DateTime {
        time::OffsetDateTime::now_utc().into()
    }

    fn solver(&self) -> Self::Solver {
        ClarabelSolver::default()
    }

    fn generate_demand_id(&self, _data: &Self::DemandData) -> DemandId {
        uuid::Uuid::new_v4().into()
    }

    fn generate_portfolio_id(&self, _data: &Self::PortfolioData) -> PortfolioId {
        uuid::Uuid::new_v4().into()
    }

    fn generate_product_id(&self, _data: &Self::ProductData) -> ProductId {
        uuid::Uuid::new_v4().into()
    }

    async fn can_create_bid(&self, _context: &Self::Context) -> Option<BidderId> {
        None
    }

    async fn can_query_bid(&self, _context: &Self::Context) -> Vec<BidderId> {
        Vec::new()
    }

    async fn can_read_bid(&self, _context: &Self::Context, _bidder_id: BidderId) -> bool {
        false
    }

    async fn can_update_bid(&self, _context: &Self::Context, _bidder_id: BidderId) -> bool {
        false
    }

    async fn can_view_products(&self, _context: &Self::Context) -> bool {
        true
    }

    async fn can_manage_products(&self, _context: &Self::Context) -> bool {
        false
    }

    async fn can_run_batch(&self, _context: &Self::Context) -> bool {
        false
    }
}
