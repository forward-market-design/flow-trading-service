use crate::{AppState, Now, utils::Bidder};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fts_core::{
    models::{CostHistoryRecord, CostId, DateTimeRangeQuery, DateTimeRangeResponse},
    ports::{CostFailure, CostRepository, MarketRepository},
};
use tracing::{Level, event};

#[utoipa::path(
    get,
    path = "/v0/submissions/{bidder_id}/costs/{cost_id}/history",
    responses(
        (status = OK, body = crate::openapi::ExampleCostHistoryResponse),
        (status = UNAUTHORIZED), // no jwt token, handled by extractor
        (status = FORBIDDEN), // not allowed to see cost
        (status = NOT_FOUND), // no cost by that id
        (status = INTERNAL_SERVER_ERROR)
    ),
    params(
        ("cost_id" = CostId, description = "Unique identifier of the cost"),
        DateTimeRangeQuery
    ),
    tags = ["costs", "history"]
)]
/// Query for any matching results
pub async fn get_history<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Now(now): Now,
    Bidder(bidder_id): Bidder,
    Path(cost_id): Path<CostId>,
    Query(query): Query<DateTimeRangeQuery>,
) -> Result<Json<DateTimeRangeResponse<CostHistoryRecord>>, StatusCode> {
    // TODO: make the limit a server config variable
    let result = CostRepository::get_history(&state.market, bidder_id, cost_id, query, 100)
        .await
        .map_err(|error| {
            event!(Level::ERROR, ?error);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map_err(|err| match err {
            // For some reason the implementation does not consider the bidder to have adequate permissions
            CostFailure::AccessDenied => StatusCode::FORBIDDEN,
            // The cost does not exist
            CostFailure::DoesNotExist => StatusCode::NOT_FOUND,
            // This value should probably never be returned.
            error => {
                event!(Level::ERROR, ?error, "unexpected failure");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(Json(result))
}
