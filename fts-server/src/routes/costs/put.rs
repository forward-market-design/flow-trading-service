use super::CostParams;
use crate::{AppState, Now, utils::Bidder};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use fts_core::{
    models::{CostData, CostId, CostRecord, Group},
    ports::{CostFailure, CostRepository, MarketRepository},
};
use serde::Deserialize;
use tracing::{Level, event};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[serde(untagged)]
pub enum PutCostDto {
    Create {
        #[schema(value_type = std::collections::HashMap<fts_core::models::AuthId, f64>)]
        group: Group,
        data: CostData,
    },
    Update {
        data: CostData,
    },
    Delete,
}

#[utoipa::path(
    put,
    path = "/v0/costs/{cost_id}",
    request_body = PutCostDto,
    responses(
        (status = OK, body = CostRecord),
        (status = UNAUTHORIZED), // no jwt token, handled by extractor
        (status = FORBIDDEN), // not allowed to see cost
        (status = NOT_FOUND), // no cost by that id
        (status = BAD_REQUEST), // JSON failure, handled by Axum
        (status = UNSUPPORTED_MEDIA_TYPE), // JSON failure, handled by Axum
        (status = UNPROCESSABLE_ENTITY), // JSON failure, handled by Axum
        (status = CONFLICT), // When creating new cost, requested authid conflicts with existing one
        (status = INTERNAL_SERVER_ERROR)
    ),
    params(
        ("cost_id" = CostId, description = "Unique identifier of the cost"),
        CostParams
    ),
    tags = ["costs"]
)]
/// Replace the cost with a new one
pub async fn put_cost<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Now(now): Now,
    Bidder(bidder_id): Bidder,
    Path(cost_id): Path<CostId>,
    Query(params): Query<CostParams>,
    Json(input): Json<PutCostDto>,
) -> Result<Json<CostRecord>, StatusCode> {
    let record = (match input {
        PutCostDto::Create { group, data } => {
            CostRepository::create(
                &state.market,
                bidder_id,
                Some(cost_id),
                group.into_iter(),
                data,
                now,
                params.group,
            )
            .await
        }
        PutCostDto::Update { data } => {
            CostRepository::update(&state.market, bidder_id, cost_id, data, now, params.group).await
        }
        PutCostDto::Delete => {
            CostRepository::delete(&state.market, bidder_id, cost_id, now, params.group).await
        }
    })
    .map_err(|err| {
        event!(Level::ERROR, error = ?err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .map_err(|err| match err {
        // For some reason the implementation does not consider the bidder to have adequate permissions
        CostFailure::AccessDenied => StatusCode::FORBIDDEN,
        // The provided auth_id is already in use
        CostFailure::IdConflict => StatusCode::CONFLICT,
        // This value should never be returned
        error => {
            event!(Level::ERROR, ?error, "unexpected failure");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(record))
}
