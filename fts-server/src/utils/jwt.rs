use crate::AppState;
use axum::extract::FromRef;
use fts_core::ports::MarketRepository;
use jwt_simple::{
    algorithms::{HS256Key, MACLike},
    claims::JWTClaims,
};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct JWTVerifier(HS256Key);

impl JWTVerifier {
    pub fn from(secret: &str) -> Self {
        Self(HS256Key::from_bytes(secret.as_bytes()))
    }

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

#[derive(Serialize, Deserialize)]
pub struct CustomJWTClaims {
    #[serde(default)]
    pub admin: bool,
}
