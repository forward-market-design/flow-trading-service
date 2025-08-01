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
use rand::RngCore;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Placeholder demand data structure.
///
/// This represents application-specific data that can be attached to demand entities.
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct DemandData {
    name: String,
}

/// Placeholder portfolio data structure.
///
/// This represents application-specific data that can be attached to portfolio entities.
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct PortfolioData {
    name: String,
}

/// The various types of products
#[repr(u32)]
#[derive(Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "u32", into = "u32")]
pub enum ProductKind {
    /// A forward product, settled as a derivative of the actual
    Forward = 0,
    /// An option on a product
    Option = 1,
}

impl TryFrom<u32> for ProductKind {
    type Error = u32;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        // manually keep this in sync with the variants
        if value > 1 {
            Err(value)
        } else {
            Ok(unsafe { std::mem::transmute(value) })
        }
    }
}

impl Into<u32> for ProductKind {
    fn into(self) -> u32 {
        unsafe { std::mem::transmute(self) }
    }
}

/// Defines a product characterized by a "kind" and an interval of time ("from", "thru").
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ProductData {
    #[schemars(schema_with = "time_schema")]
    #[serde(with = "time::serde::rfc3339")]
    from: time::OffsetDateTime,
    #[schemars(schema_with = "time_schema")]
    #[serde(with = "time::serde::rfc3339")]
    thru: time::OffsetDateTime,
    kind: ProductKind,
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

    fn generate_demand_id(&self, _data: &DemandData) -> (DemandId, DateTime) {
        let now = time::OffsetDateTime::now_utc();

        let id = {
            let rng56 = rand::rng().next_u64() >> 8; // 56 random bits

            // Current timestamp, partitioned into (48, 12, 4) bits and splatted into a V8 pattern with id tag
            let now = now.unix_timestamp() as u64;
            let now48 = 0xffff_ffff_ffff_0000 & now;
            let now12 = (0xfff0 & now) >> 4;
            let now04 = (0x000f & now) << 56;

            let hi = 0x0000_0000_0000_8000 | now48 | now12;
            let lo = 0x9000_0000_0000_0000 | now04 | rng56;

            Uuid::from_u64_pair(hi, lo)
        };

        (id.into(), now.into())
    }

    fn generate_portfolio_id(&self, _data: &PortfolioData) -> (PortfolioId, DateTime) {
        let now = time::OffsetDateTime::now_utc();

        let id = {
            let rng56 = rand::rng().next_u64() >> 8; // 56 random bits

            // Current timestamp, partitioned into (48, 12, 4) bits and splatted into a V8 pattern with id tag
            let now = now.unix_timestamp() as u64;
            let now48 = 0xffff_ffff_ffff_0000 & now;
            let now12 = (0xfff0 & now) >> 4;
            let now04 = (0x000f & now) << 56;

            let hi = 0x0000_0000_0000_8000 | now48 | now12;
            let lo = 0xa000_0000_0000_0000 | now04 | rng56;
            Uuid::from_u64_pair(hi, lo)
        };

        (id.into(), now.into())
    }

    fn generate_product_id(&self, data: &ProductData) -> (ProductId, DateTime) {
        // Starting time, partitioned into (48, 12, 4) bits and splatted into a V8 pattern with id tag
        let now = data.from.unix_timestamp() as u64;
        let now48 = 0xffff_ffff_ffff_0000 & now;
        let now12 = (0xfff0 & now) >> 4;
        let now04 = (0x000f & now) << 56;

        let duration = (((data.thru - data.from).whole_seconds() as u64) & 0xffff_ffff) << 24; // first 8 zero, middle 32 useful, last 24 zero
        let kind = (<ProductKind as Into<u32>>::into(data.kind) as u64) & 0x00ff_ffff;

        let hi = 0x0000_0000_0000_8000 | now48 | now12;
        let lo = 0xb000_0000_0000_0000 | now04 | duration | kind;
        (Uuid::from_u64_pair(hi, lo).into(), self.now())
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
