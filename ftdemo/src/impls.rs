//! Application implementation with JWT-based authorization.
//!
//! This module provides the concrete implementation of the Application trait,
//! integrating all components of the flow trading system with JWT-based
//! authorization.

use fts_core::ports::Application;
use fts_solver::clarabel::ClarabelSolver;
use fts_sqlite::{
    Db,
    types::{BidderId, DateTime, DemandId, PortfolioId, ProductId},
};
use headers::{Authorization, authorization::Bearer};
use jwt_simple::{
    claims::JWTClaims,
    prelude::{HS256Key, MACLike},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Placeholder demand data structure.
///
/// This represents application-specific data that can be attached to demand entities.
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct DemandData;

/// Placeholder portfolio data structure.
///
/// This represents application-specific data that can be attached to portfolio entities.
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct PortfolioData;

/// Defines a product characterized by a "kind" and an interval of time ("from", "thru").
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ProductData {
    kind: String,
    #[schemars(schema_with = "time_schema")]
    #[serde(with = "time::serde::rfc3339")]
    from: time::OffsetDateTime,
    #[schemars(schema_with = "time_schema")]
    #[serde(with = "time::serde::rfc3339")]
    thru: time::OffsetDateTime,
}

/// Main application implementation combining all system components.
///
/// This struct implements the Application trait and provides the integration point
/// for the database, authorization, and business logic. It uses JWT tokens for
/// authorization decisions.
#[derive(Clone)]
pub struct DemoApp {
    /// Database connection for persistent storage
    pub db: Db,
    /// HMAC key for JWT token verification
    pub key: HS256Key,
}

impl DemoApp {
    /// Extract and verify JWT claims from the authorization header.
    fn claims(&self, context: &Authorization<Bearer>) -> Option<JWTClaims<CustomJWTClaims>> {
        let token = context.0.token();
        self.key.verify_token::<CustomJWTClaims>(token, None).ok()
    }
}

impl Application for DemoApp {
    type Context = Authorization<Bearer>;
    type DemandData = DemandData;
    type PortfolioData = PortfolioData;
    type ProductData = ProductData;
    type Repository = Db;
    type Solver = ClarabelSolver<DemandId, PortfolioId, ProductId>;

    fn database(&self) -> &Self::Repository {
        &self.db
    }

    fn solver(&self) -> Self::Solver {
        ClarabelSolver::default()
    }

    fn now(&self) -> DateTime {
        time::OffsetDateTime::now_utc().into()
    }

    fn generate_demand_id(&self, _data: &DemandData) -> DemandId {
        uuid::Uuid::new_v4().into()
    }

    fn generate_portfolio_id(&self, _data: &PortfolioData) -> PortfolioId {
        uuid::Uuid::new_v4().into()
    }

    fn generate_product_id(&self, _data: &ProductData) -> ProductId {
        uuid::Uuid::new_v4().into()
        // // The other id generators make random ids, but this one creates
        // // a deterministic id based on the content of the product data, roughly
        // // (FROM)(THRU)(KIND)
        // // so that "nearby" products are sorted together.
        // let pattern = ((data.from.unix_timestamp() as u128) << 65)
        //     | ((data.thru.unix_timestamp() << 2) as u128)
        //     | match data.kind {
        //         ProductKind::Forward => 1u128,
        //         ProductKind::Option => 2u128,
        //     };

        // // TODO: this is wrong, (v8 will overwrite a few bytes)
        // uuid::Uuid::new_v8(pattern.to_le_bytes()).into()
    }

    async fn can_create_bid(&self, context: &Self::Context) -> Option<BidderId> {
        // The demo app takes the standard sub: claim to be the bidder id
        Some(self.claims(context)?.subject?.parse().ok()?)
    }

    async fn can_query_bid(&self, context: &Self::Context) -> Vec<BidderId> {
        // The demo app takes the standard sub: claim to be the single allowed bidder id
        self.claims(context)
            .and_then(|claims| claims.subject)
            .and_then(|sub| sub.parse().ok())
            .map(|bidder_id| vec![bidder_id])
            .unwrap_or_default()
    }

    async fn can_update_bid(&self, context: &Self::Context, bidder_id: BidderId) -> bool {
        // The demo app compares the sub: claim against the specified bidder_id
        self.claims(context)
            .and_then(|claims| claims.subject)
            .and_then(|sub| sub.parse::<BidderId>().ok())
            .map(|claim_bidder| claim_bidder == bidder_id)
            .unwrap_or(false)
    }

    async fn can_read_bid(&self, context: &Self::Context, bidder_id: BidderId) -> bool {
        // The demo app compares the sub: claim against the specified bidder_id
        self.claims(context)
            .and_then(|claims| claims.subject)
            .and_then(|sub| sub.parse::<BidderId>().ok())
            .map(|claim_bidder| claim_bidder == bidder_id)
            .unwrap_or(false)
    }

    async fn can_view_products(&self, context: &Self::Context) -> bool {
        // anybody with a valid JWT can view products
        // Note: it would be reasonable to say no JWT required!
        // But, if we found ourselves needed to rate-limit a token, this
        // would be a good place to implement that logic.
        self.claims(context).is_some()
    }

    async fn can_manage_products(&self, context: &Self::Context) -> bool {
        // managing products requires an `admin: true` custom claim
        self.claims(context)
            .and_then(|claims| Some(claims.custom.admin))
            .unwrap_or(false)
    }

    async fn can_run_batch(&self, context: &Self::Context) -> bool {
        // running a batch requires an `admin: true` custom claim
        self.claims(context)
            .and_then(|claims| Some(claims.custom.admin))
            .unwrap_or(false)
    }
}

/// Helper function to generate JSON schema for time::OffsetDateTime.
///
/// This is needed because the schemars crate doesn't have built-in support
/// for the time crate's OffsetDateTime type.
fn time_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "string",
        "format": "date-time",
    })
}

/// Custom claims structure for JWT tokens.
///
/// Contains application-specific claims beyond standard JWT claims.
#[derive(Serialize, Deserialize)]
pub struct CustomJWTClaims {
    /// Indicates whether the token holder has admin privileges.
    #[serde(default)]
    pub admin: bool,
}
