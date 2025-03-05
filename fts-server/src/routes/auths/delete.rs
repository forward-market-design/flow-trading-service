use super::AuthParams;
use crate::{AppState, Now, utils::Bidder};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fts_core::{
    models::{AuthId, AuthRecord},
    ports::{AuthFailure, AuthRepository, MarketRepository},
};
use tracing::{Level, event};

#[utoipa::path(
    delete,
    path = "/v0/auths/{auth_id}",
    responses(
        (status = OK, body = AuthRecord),
        (status = UNAUTHORIZED), // no jwt token, handled by extractor
        (status = FORBIDDEN), // not allowed to see auth
        (status = NOT_FOUND), // no auth by that id
        (status = INTERNAL_SERVER_ERROR)
    ),
    params(
        ("auth_id" = AuthId, Path, description = "Unique identifier of the authorization"),
        ("portfolio" = Option<String>, Query, description = "implementation-dependent portfolio mode")
    ),
    tags = ["auths"]
)]
/// "Delete" the authorization (that is, set its data to null)
pub async fn delete_auth<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Now(now): Now,
    Bidder(bidder_id): Bidder,
    Path(auth_id): Path<AuthId>,
    Query(params): Query<AuthParams<T::PortfolioOptions>>,
) -> Result<Json<AuthRecord>, StatusCode> {
    let record = AuthRepository::delete(&state.market, bidder_id, auth_id, now, params.portfolio)
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
            err => {
                event!(Level::ERROR, "unexpected failure in auth delete: {err:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(Json(record))
}
