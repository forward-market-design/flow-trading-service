use super::CostParams;
use crate::{AppState, Now, utils::Bidder};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fts_core::{
    models::{CostId, CostRecord},
    ports::{CostFailure, CostRepository, MarketRepository},
};
use tracing::{Level, event};

#[utoipa::path(
    delete,
    path = "/v0/costs/{cost_id}",
    responses(
        (status = OK, body = CostRecord),
        (status = UNAUTHORIZED), // no jwt token, handled by extractor
        (status = FORBIDDEN), // not allowed to see cost
        (status = NOT_FOUND), // no cost by that id
        (status = INTERNAL_SERVER_ERROR)
    ),
    params(
        ("cost_id" = CostId, description = "Unique identifier of the cost"),
        CostParams
    ),
    tags = ["costs"]
)]
/// Clear the cost
pub async fn delete_cost<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Now(now): Now,
    Bidder(bidder_id): Bidder,
    Path(cost_id): Path<CostId>,
    Query(params): Query<CostParams>,
) -> Result<Json<CostRecord>, StatusCode> {
    let record = CostRepository::delete(&state.market, bidder_id, cost_id, now, params.group)
        .await
        .map_err(|err| {
            event!(Level::ERROR, error = ?err);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map_err(|err| match err {
            // For some reason the implementation does not consider the bidder to have adequate permissions
            CostFailure::AccessDenied => StatusCode::FORBIDDEN,
            // The cost does not exist
            CostFailure::DoesNotExist => StatusCode::NOT_FOUND,
            error => {
                event!(Level::ERROR, ?error, "unexpected failure");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(Json(record))
}
