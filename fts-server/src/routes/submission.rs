use crate::{AppState, Now, utils::Bidder};
use axum::{
    Json, Router,
    extract::{FromRequestParts, Path, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing,
};
use fts_core::{
    models::{BidderId, SubmissionRecord},
    ports::{MarketRepository, SubmissionDto, SubmissionRepository},
};
use tracing::{Level, event};

pub fn router<T: MarketRepository>(state: AppState<T>) -> Router<AppState<T>> {
    Router::new()
        .route(
            "/{bidder_id}",
            routing::get(get_submission)
                .put(put_submission)
                .delete(delete_submission),
        )
        // Automatically check the bidder consistency for each endpoint
        .route_layer(middleware::from_fn_with_state(
            state,
            verify_bidder::<AppState<T>>,
        ))
}

async fn verify_bidder<S>(
    Bidder(auth_bidder_id): Bidder,
    Path(url_bidder_id): Path<BidderId>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode>
where
    Bidder: FromRequestParts<S>,
{
    if auth_bidder_id == url_bidder_id {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

#[utoipa::path(
    get,
    path = "/v0/submissions/{bidder_id}",
    responses(
        (status = OK, body = SubmissionRecord),
        (status = INTERNAL_SERVER_ERROR)
    ),
    params(
        ("bidder_id" = BidderId, description = "Unique identifier of the bidder")
    ),
    tags = ["submissions"]
)]
async fn get_submission<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Path(bidder_id): Path<BidderId>,
    Now(now): Now,
) -> Result<Json<SubmissionRecord>, StatusCode> {
    let result = SubmissionRepository::get_submission(&state.market, bidder_id, now)
        .await
        .map_err(|error| {
            event!(Level::ERROR, ?error);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(result))
}

#[utoipa::path(
    put,
    path = "/v0/submissions/{bidder_id}",
    request_body = SubmissionDto,
    responses(
        (status = OK, body = SubmissionRecord),
        (status = INTERNAL_SERVER_ERROR)
    ),
    params(
        ("bidder_id" = BidderId, description = "Unique identifier of the bidder")
    ),
    tags = ["submissions"]
)]
async fn put_submission<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Path(bidder_id): Path<BidderId>,
    Now(now): Now,
    Json(input): Json<SubmissionDto>,
) -> Result<Json<SubmissionRecord>, StatusCode> {
    let result = SubmissionRepository::set_submission(&state.market, bidder_id, input, now)
        .await
        .map_err(|error| {
            event!(Level::ERROR, ?error);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(result))
}

#[utoipa::path(
    delete,
    path = "/v0/submissions/{bidder_id}",
    responses(
        (status = OK, body = OffsetDateTime, description = "Just returns the version timestamp"),
        (status = INTERNAL_SERVER_ERROR)
    ),
    params(
        ("bidder_id" = BidderId, description = "Unique identifier of the bidder")
    ),
    tags = ["submissions"]
)]
async fn delete_submission<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Path(bidder_id): Path<BidderId>,
    Now(now): Now,
) -> Result<impl IntoResponse, StatusCode> {
    let result = SubmissionRepository::set_submission(
        &state.market,
        bidder_id,
        SubmissionDto {
            auths: Vec::new(),
            costs: Vec::new(),
        },
        now,
    )
    .await
    .map_err(|error| {
        event!(Level::ERROR, ?error);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(result))
}
