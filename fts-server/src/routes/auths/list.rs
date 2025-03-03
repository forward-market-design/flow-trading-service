#![allow(unused)]
// ^ TODO: remove this

use crate::{AppState, Now, utils::Bidder};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fts_core::{
    models::{AuthId, AuthRecord, DateTimeRangeQuery, DateTimeRangeResponse},
    ports::MarketRepository,
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct AuthListItem {
    pub auth_id: AuthId,
    // what other properties should go here?
}

#[utoipa::path(
    get,
    path = "/v0/auths",
    responses(
        (status = OK, body = DateTimeRangeResponse<AuthListItem>),
        (status = UNAUTHORIZED), // no jwt token, handled by extractor
        // (status = FORBIDDEN), // not allowed to see auth
        // (status = NOT_FOUND), // no auth by that id
        (status = INTERNAL_SERVER_ERROR)
    ),
    tags = ["auths"]
)]
/// Retrieve the requested authorization, if possible
pub async fn list_auths<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Now(now): Now,
    Bidder(bidder_id): Bidder,
    Path(auth_id): Path<AuthId>,
    Query(query): Query<DateTimeRangeQuery>,
) -> Result<Json<DateTimeRangeResponse<AuthListItem>>, StatusCode> {
    // TODO: an endpoint that queries all of my auths, active or not
    Err(StatusCode::NOT_IMPLEMENTED)
}
