use axum::{extract::FromRequestParts, http::request::Parts};
use time::OffsetDateTime;

/// A wrapper for the current time in UTC.
///
/// This struct provides the current UTC time as an extractor,
/// allowing handlers to easily access a consistent timestamp
/// during request processing.
pub struct Now(pub OffsetDateTime);

impl<S> FromRequestParts<S> for Now
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(_: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(Now(time::OffsetDateTime::now_utc()))
    }
}
