use super::AuthParams;
use crate::{AppState, Now, utils::Bidder};
use axum::{
    Json,
    extract::{Path, Query, State},
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
#[serde(untagged)]
pub enum PutAuthDto {
    Create {
        portfolio: Portfolio,
        data: AuthData,
    },
    Update {
        data: AuthData,
    },
    Delete,
}

#[utoipa::path(
    put,
    path = "/v0/auths/{auth_id}",
    request_body = PutAuthDto,
    responses(
        (status = OK, body = AuthRecord),
        (status = UNAUTHORIZED), // no jwt token, handled by extractor
        (status = FORBIDDEN), // not allowed to see auth
        (status = NOT_FOUND), // no auth by that id
        (status = BAD_REQUEST), // JSON failure, handled by Axum
        (status = UNSUPPORTED_MEDIA_TYPE), // JSON failure, handled by Axum
        (status = UNPROCESSABLE_ENTITY), // JSON failure, handled by Axum
        (status = CONFLICT), // When creating new auth, requested authid conflicts with existing one
        (status = INTERNAL_SERVER_ERROR)
    ),
    params(
        ("auth_id" = AuthId, Path, description = "Unique identifier of the authorization"),
        ("portfolio" = Option<String>, Query, description = "implementation-dependent portfolio mode")
    ),
    tags = ["auths"]
)]
/// Replace the authorization with a new one
pub async fn put_auth<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Now(now): Now,
    Bidder(bidder_id): Bidder,
    Path(auth_id): Path<AuthId>,
    Query(params): Query<AuthParams<T::PortfolioOptions>>,
    Json(input): Json<PutAuthDto>,
) -> Result<Json<AuthRecord>, StatusCode> {
    let record = (match input {
        PutAuthDto::Create { portfolio, data } => {
            AuthRepository::create(
                &state.market,
                bidder_id,
                Some(auth_id),
                portfolio.into_iter(),
                data,
                now,
                params.portfolio,
            )
            .await
        }
        PutAuthDto::Update { data } => {
            AuthRepository::update(
                &state.market,
                bidder_id,
                auth_id,
                data,
                now,
                params.portfolio,
            )
            .await
        }
        PutAuthDto::Delete => {
            AuthRepository::delete(&state.market, bidder_id, auth_id, now, params.portfolio).await
        }
    })
    .map_err(|err| {
        event!(Level::ERROR, error = ?err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .map_err(|err| match err {
        // For some reason the implementation does not consider the bidder to have adequate permissions
        AuthFailure::AccessDenied => StatusCode::FORBIDDEN,
        // The provided auth_id is already in use
        AuthFailure::IdConflict => StatusCode::CONFLICT,
        // This should never be returned, since we explicitly support creating new records from PUT
        error => {
            event!(Level::ERROR, ?error, "unexpected failure");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(record))
}
