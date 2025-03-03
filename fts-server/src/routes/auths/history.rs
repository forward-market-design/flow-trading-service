use crate::{AppState, utils::Bidder};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fts_core::{
    models::{AuthHistoryRecord, AuthId, DateTimeRangeQuery, DateTimeRangeResponse},
    ports::{AuthFailure, AuthRepository, MarketRepository},
};
use tracing::{Level, event};

#[utoipa::path(
    get,
    path = "/v0/auths/{auth_id}/history",
    responses(
        (status = OK, body = crate::openapi::ExampleAuthHistoryResponse),
        (status = UNAUTHORIZED), // no jwt token, handled by extractor
        (status = FORBIDDEN), // not allowed to see auth
        (status = NOT_FOUND), // no auth by that id
        (status = INTERNAL_SERVER_ERROR)
    ),
    params(
        ("auth_id" = AuthId, description = "Unique identifier of the authorization"),
        DateTimeRangeQuery
    ),
    tags = ["auths", "history"]
)]
/// Query for any matching results
pub async fn get_history<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Bidder(bidder_id): Bidder,
    Path(auth_id): Path<AuthId>,
    Query(query): Query<DateTimeRangeQuery>,
) -> Result<Json<DateTimeRangeResponse<AuthHistoryRecord>>, StatusCode> {
    // TODO: make the limit a server config variable
    let result = AuthRepository::get_history(&state.market, bidder_id, auth_id, query, 100)
        .await
        .map_err(|err| {
            event!(Level::ERROR, error = ?err);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map_err(|err| match err {
            // For some reason the implementation does not consider the bidder to have adequate permissions
            AuthFailure::AccessDenied => StatusCode::FORBIDDEN,
            // The auth does not exist
            AuthFailure::DoesNotExist => StatusCode::NOT_FOUND,
            // This value should probably never be returned.
            error => {
                event!(Level::ERROR, ?error, "unexpected failure");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(Json(result))
}
