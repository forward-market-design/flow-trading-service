use super::AuthParams;
use crate::{AppState, Now, utils::Bidder};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fts_core::{
    models::{AuthData, AuthId, AuthRecord, Portfolio},
    ports::{AuthFailure, AuthRepository, MarketRepository},
};
use serde::Deserialize;
use tracing::{Level, event};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct PostAuthDto {
    auth_id: Option<AuthId>,
    #[schema(value_type = std::collections::HashMap<fts_core::models::ProductId, f64>)]
    portfolio: Portfolio,
    data: AuthData,
}

#[utoipa::path(
    post,
    path = "/v0/auths",
    request_body = PostAuthDto,
    responses(
        (status = OK, body = AuthRecord),
        (status = UNAUTHORIZED), // no jwt token, handled by extractor
        (status = BAD_REQUEST), // JSON failure, handled by Axum
        (status = UNSUPPORTED_MEDIA_TYPE), // JSON failure, handled by Axum
        (status = UNPROCESSABLE_ENTITY), // JSON failure, handled by Axum
        (status = INTERNAL_SERVER_ERROR)
    ),
    tags = ["auths"]
)]
/// Create a new portfolio with optional inline authorization.
pub async fn post_auth<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Now(now): Now,
    Bidder(bidder_id): Bidder,
    Query(params): Query<AuthParams>,
    Json(PostAuthDto {
        auth_id,
        portfolio,
        data,
    }): Json<PostAuthDto>,
) -> Result<Json<AuthRecord>, StatusCode> {
    let record = AuthRepository::create(
        &state.market,
        bidder_id,
        auth_id,
        portfolio.into_iter(),
        data,
        now,
        params.portfolio,
    )
    .await
    .map_err(|error| {
        event!(Level::ERROR, ?error);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .map_err(|err| match err {
        // For some reason the implementation does not consider the bidder to have adequate permissions
        AuthFailure::AccessDenied => StatusCode::FORBIDDEN,
        // This value should probably never be returned.
        AuthFailure::IdConflict => StatusCode::CONFLICT,
        // This value should never be returned
        error => {
            event!(Level::ERROR, ?error, "unexpected failure");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(record))
}
