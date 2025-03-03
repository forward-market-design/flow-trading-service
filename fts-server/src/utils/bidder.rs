use super::JWTVerifier;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use fts_core::models::BidderId;
use uuid::Uuid;

/// An authenticated bidder identity.
///
/// This extractor verifies the JWT token from the request headers
/// and extracts the bidder ID. It's used to authenticate and
/// authorize operations specific to a bidder.
pub struct Bidder(pub BidderId);

impl<S> FromRequestParts<S> for Bidder
where
    S: Send + Sync,
    JWTVerifier: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract the bearer token, returning 401 if not provided
        let TypedHeader(auth) =
            Option::<TypedHeader<Authorization<Bearer>>>::from_request_parts(parts, state)
                .await
                .unwrap()
                .ok_or(StatusCode::UNAUTHORIZED)?;

        // Extract the claims from the bearer token, returning 401 if any errors occur
        let claims = JWTVerifier::from_ref(state)
            .claims(auth.token())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        // Extract the BidderId from the claims, returning 401 if subject cannot be parsed as UUID
        let subject = claims.subject.ok_or(StatusCode::UNAUTHORIZED)?;
        let bidder_id = Uuid::try_parse(&subject).map_err(|_| StatusCode::UNAUTHORIZED)?;
        Ok(Self(bidder_id.into()))
    }
}
