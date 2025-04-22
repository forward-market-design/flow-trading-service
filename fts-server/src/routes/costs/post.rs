use crate::{AppState, Now, utils::Bidder};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use fts_core::{
    models::{CostData, CostId, CostRecord, Group},
    ports::{CostFailure, CostRepository, MarketRepository},
};
use serde::Deserialize;
use tracing::{Level, event};
use utoipa::ToSchema;

use super::CostParams;

#[derive(Deserialize, ToSchema)]
pub struct PostCostDto {
    cost_id: Option<CostId>,
    #[schema(value_type = std::collections::HashMap<fts_core::models::AuthId, f64>)]
    group: Group,
    data: CostData,
}

#[utoipa::path(
    post,
    path = "/v0/costs",
    request_body = PostCostDto,
    responses(
        (status = CREATED, body = CostRecord),
        (status = UNAUTHORIZED), // no jwt token, handled by extractor
        (status = BAD_REQUEST), // JSON failure, handled by Axum
        (status = UNSUPPORTED_MEDIA_TYPE), // JSON failure, handled by Axum
        (status = UNPROCESSABLE_ENTITY), // JSON failure, handled by Axum
        (status = INTERNAL_SERVER_ERROR)
    ),
    tags = ["costs"]
)]
/// Create a new cost
pub async fn post_cost<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Now(now): Now,
    Bidder(bidder_id): Bidder,
    Query(params): Query<CostParams>,
    Json(PostCostDto {
        cost_id,
        group,
        data,
    }): Json<PostCostDto>,
) -> Result<Json<CostRecord>, StatusCode> {
    let record = CostRepository::create(
        &state.market,
        bidder_id,
        cost_id,
        group.into_iter(),
        data,
        now,
        params.group,
    )
    .await
    .map_err(|error| {
        event!(Level::ERROR, ?error);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .map_err(|err| match err {
        // For some reason the implementation does not consider the bidder to have adequate permissions
        CostFailure::AccessDenied => StatusCode::FORBIDDEN,
        // This value should probably never be returned.
        CostFailure::IdConflict => StatusCode::CONFLICT,
        // This value should never be returned
        error => {
            event!(Level::ERROR, ?error, "unexpected failure");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(record))
}
