use super::JWTVerifier;
use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};

/// An authenticated administrator identity.
///
/// This extractor verifies the JWT token from the request headers
/// and confirms that the user has administrator privileges.
/// It's used to authorize administrative operations.
pub struct Admin;

impl<S> FromRequestParts<S> for Admin
where
    S: Send + Sync,
    JWTVerifier: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract the bearer token
        let header = Option::<TypedHeader<Authorization<Bearer>>>::from_request_parts(parts, state)
            .await
            .unwrap()
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let TypedHeader(auth) = header;

        let jwt = JWTVerifier::from_ref(state);

        // Process the claims. According to simple-jwt docs, this will automatically
        // check and verify all the things a responsible implementation should.
        let admin = jwt
            .claims(auth.token())
            .ok_or(StatusCode::BAD_REQUEST)?
            .custom
            .admin;
        if admin {
            Ok(Self)
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
}
