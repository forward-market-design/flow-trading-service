#![allow(unused)]
// ^ TODO: remove this

use crate::{AppState, Now, utils::Bidder};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fts_core::{
    models::{CostId, CostRecord, DateTimeRangeQuery, DateTimeRangeResponse},
    ports::MarketRepository,
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct CostListItem {
    pub cost_id: CostId,
    // what other properties should go here?
}

#[utoipa::path(
    get,
    path = "/v0/costs",
    responses(
        (status = OK, body = DateTimeRangeResponse<CostListItem>),
        (status = UNAUTHORIZED), // no jwt token, handled by extractor
        // (status = FORBIDDEN), // not allowed to see auth
        // (status = NOT_FOUND), // no auth by that id
        (status = INTERNAL_SERVER_ERROR)
    ),
    tags = ["costs"]
)]
/// Retrieve the requested authorization, if possible
pub async fn list_costs<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Now(now): Now,
    Bidder(bidder_id): Bidder,
    Path(cost_id): Path<CostId>,
    Query(query): Query<DateTimeRangeQuery>,
) -> Result<Json<DateTimeRangeResponse<CostListItem>>, StatusCode> {
    // TODO: an endpoint that queries all of my costs, active or not
    Err(StatusCode::NOT_IMPLEMENTED)
}
