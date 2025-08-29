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

#[cfg(test)]
mod uuid_v8_tests {
    use super::*;

    // ==============================================================
    // UUID v8 Custom Layout
    // ==============================================================
    // These tests document and verify the custom bit-packing scheme
    // used to embed:
    //   * A timestamp (split across three segments: 48 + 12 + 4 bits)
    //   * The UUID version (v8) and RFC 4122 variant bits
    //   * A 4-bit "namespace" nibble distinguishing entity type
    //   * Either randomness (demand / portfolio) or semantic payload
    //     (duration + kind) for products.
    //
    // Why split the timestamp? We want: chronological ordering to rely
    // on the high word, while still carving out room for version bits
    // and an entity discriminator without hashing or extra lookups.
    // The bottom 16 timestamp bits are decomposed so that:
    //   - Bits 4..15 go into low 12 bits of the high 64-bit word
    //   - Bits 0..3 go into bits 56..59 of the low 64-bit word
    // The high 48 bits stay where they are. Reassembly = OR with shifts.
    // -------------------------------------------------------------
    // For reference, here is the overall field/bit layout of an UUID v8:
    // See: https://www.rfc-editor.org/rfc/rfc9562.html#name-uuid-version-8
    //
    //  0                   1                   2                   3
    //  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                           custom_a                            |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |          custom_a             |  ver  |       custom_b        |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |var|                       custom_c                            |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // |                           custom_c                            |
    // +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    // ==============================================================

    /// Extract (version, variant2bits, namespace nibble) from a UUID
    fn extract_meta(uuid: Uuid) -> (u8, u8, u8) {
        let (hi, lo) = uuid.as_u64_pair();
        let version = ((hi >> 12) & 0xF) as u8; // bits 12..15 of hi
        let variant = ((lo >> 62) & 0x3) as u8; // top two bits of lo
        let namespace = ((lo >> 60) & 0xF) as u8; // next 4 bits
        (version, variant, namespace)
    }

    /// Split a timestamp into the three stored fragments matching implementation.
    fn split_timestamp(ts: u64) -> (u64, u64, u64) {
        let high48 = ts & 0xffff_ffff_ffff_0000; // upper 48 bits remain in place
        let mid12 = (ts & 0x0000_0000_0000_fff0) >> 4; // bits 4..15 -> 0..11
        let low4 = ts & 0xF; // bits 0..3
        (high48, mid12, low4)
    }

    /// Reassemble timestamp from fragments (inverse of split_timestamp)
    fn reassemble_timestamp(high48: u64, mid12: u64, low4: u64) -> u64 {
        high48 | (mid12 << 4) | low4
    }

    /// Extract timestamp fragments from a generated UUID (reverse mapping)
    fn extract_timestamp_fragments(uuid: Uuid) -> (u64, u64, u64) {
        let (hi, lo) = uuid.as_u64_pair();
        let high48 = hi & 0xffff_ffff_ffff_0000;
        let mid12 = hi & 0x0fff; // original bits 4..15
        let low4 = (lo >> 56) & 0x0f; // original bits 0..3
        (high48, mid12, low4)
    }

    /// Create a test app instance for UUID generation testing
    async fn create_test_app() -> DemoApp {
        let database = fts_sqlite::Db::open(&fts_sqlite::config::SqliteConfig::default())
            .await
            .unwrap();
        DemoApp {
            db: database,
            key: jwt_simple::prelude::HS256Key::generate(),
        }
    }

    // Basic test that covers structure for all entity types + determinism
    #[tokio::test]
    async fn test_uuid_structure_and_determinism() {
        let app = create_test_app().await;

        // Demand
        let (demand_id, _) = app.generate_demand_id(&DemandData { name: "d".into() });
        let (v, var, ns) = extract_meta(demand_id.0);
        assert_eq!(v, 8, "Demand: version must be 8 (v8)");
        assert_eq!(var, 0b10, "Demand: variant must be RFC4122 (10)");
        assert_eq!(ns, 0x9, "Demand: namespace nibble 0x9");

        // Portfolio
        let (portfolio_id, _) = app.generate_portfolio_id(&PortfolioData { name: "p".into() });
        let (v, var, ns) = extract_meta(portfolio_id.0);
        assert_eq!(v, 8, "Portfolio: version 8");
        assert_eq!(var, 0b10, "Portfolio: variant 10");
        assert_eq!(ns, 0xA, "Portfolio: namespace 0xA");

        // Product (Forward) – also test determinism by generating twice
        let product_data = ProductData {
            from: time::OffsetDateTime::from_unix_timestamp(1_640_995_200).unwrap(),
            thru: time::OffsetDateTime::from_unix_timestamp(1_672_531_200).unwrap(),
            kind: ProductKind::Forward,
        };
        let (prod1, _) = app.generate_product_id(&product_data);
        let (prod2, _) = app.generate_product_id(&product_data);
        assert_eq!(
            prod1, prod2,
            "Product: deterministic ID for identical input"
        );
        let (v, var, ns) = extract_meta(prod1.0);
        assert_eq!(v, 8, "Product: version 8");
        assert_eq!(var, 0b10, "Product: variant 10");
        assert_eq!(ns, 0xB, "Product: namespace 0xB");
    }

    // Splitting a known timestamp and reassembling should be lossless
    #[test]
    fn test_timestamp_split_and_reassemble_pure() {
        let ts = 1_640_995_200u64; // 2022-01-01 00:00:00 UTC
        let (h48, m12, l4) = split_timestamp(ts);

        // Basic shape checks
        assert_eq!(
            h48 & 0xFFFF,
            0,
            "High48 segment must have low 16 bits zeroed"
        );
        assert!(m12 < (1 << 12), "Mid12 must fit in 12 bits");
        assert!(l4 < (1 << 4), "Low4 must fit in 4 bits");

        let roundtrip = reassemble_timestamp(h48, m12, l4);
        assert_eq!(roundtrip, ts, "Reassembled timestamp must match original");
    }

    // Generate a demand UUID and recover the stored timestamp.
    #[tokio::test]
    async fn test_generated_uuid_timestamp_roundtrip() {
        let app = create_test_app().await;
        let (demand_id, dt) = app.generate_demand_id(&DemandData { name: "rt".into() });
        let uuid = demand_id.0;

        let (h48, m12, l4) = extract_timestamp_fragments(uuid);
        let reconstructed = reassemble_timestamp(h48, m12, l4);
        let original_secs = {
            let odt: time::OffsetDateTime = dt.into();
            odt.unix_timestamp() as u64
        };
        assert_eq!(
            reconstructed, original_secs,
            "UUID timestamp fragments must roundtrip"
        );
    }

    // Product semantic payload test: only the low kind byte should differ between Forward and Option.
    #[tokio::test]
    async fn test_product_kind_field_isolated() {
        let app = create_test_app().await;
        let base = time::OffsetDateTime::from_unix_timestamp(1_640_995_200).unwrap();
        let duration = time::Duration::days(365);

        let forward = ProductData {
            from: base,
            thru: base + duration,
            kind: ProductKind::Forward,
        };
        let option = ProductData {
            from: base,
            thru: base + duration,
            kind: ProductKind::Option,
        };
        let (forward_id, _) = app.generate_product_id(&forward);
        let (option_id, _) = app.generate_product_id(&option);

        let forward_lo = forward_id.0.as_u64_pair().1;
        let option_lo = option_id.0.as_u64_pair().1;

        let forward_kind = forward_lo & 0x00ff_ffff; // low 24 bits region
        let option_kind = option_lo & 0x00ff_ffff;
        assert_eq!(
            forward_kind & 0xFF,
            0,
            "Forward kind discriminant should be 0"
        );
        assert_eq!(
            option_kind & 0xFF,
            1,
            "Option kind discriminant should be 1"
        );

        let diff = forward_lo ^ option_lo; // XOR highlights differing bits
        assert!(
            diff <= 0xFF,
            "Only lowest byte should differ (got 0x{diff:x})"
        );
    }
}
