use crate::AppState;
use axum::extract::FromRef;
use fts_core::ports::MarketRepository;
use jwt_simple::{
    algorithms::{HS256Key, MACLike},
    claims::JWTClaims,
};
use serde::{Deserialize, Serialize};

/// JWT verification service.
///
/// Handles verification of JWT tokens and extraction of claims.
/// Uses HS256 for signature verification.
#[derive(Clone)]
pub struct JWTVerifier(HS256Key);

impl JWTVerifier {
    /// Creates a new JWTVerifier from a secret string.
    pub fn from(secret: &str) -> Self {
        Self(HS256Key::from_bytes(secret.as_bytes()))
    }

    /// Verifies a token and extracts its claims if valid.
    pub fn claims(&self, token: &str) -> Option<JWTClaims<CustomJWTClaims>> {
        // Process the claims. According to simple-jwt docs, this will automatically
        // check and verify all the things a responsible implementation should.
        self.0.verify_token::<CustomJWTClaims>(token, None).ok()
    }
}

impl<T: MarketRepository> FromRef<AppState<T>> for JWTVerifier {
    fn from_ref(input: &AppState<T>) -> Self {
        input.jwt.clone()
    }
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
