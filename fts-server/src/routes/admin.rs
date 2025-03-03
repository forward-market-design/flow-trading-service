pub mod solve;

use crate::{AppState, Now, openapi::ProductData, utils::Admin};
use axum::{Json, Router, extract::State, http::StatusCode, middleware, routing};
use fts_core::{models::ProductId, ports::MarketRepository};
use solve::solve_auctions;
use tracing::{Level, event};

pub fn router<T: MarketRepository>(state: AppState<T>) -> Router<AppState<T>> {
    Router::new()
        .route("/products", routing::post(define_products))
        .route("/auctions/solve", routing::post(solve_auctions))
        .route_layer(middleware::from_extractor_with_state::<Admin, AppState<T>>(
            state,
        ))
}

/// Define new products for the marketplace.
///
/// This endpoint defines new products based on the provided data and returns the newly created ids.
#[utoipa::path(
    post,
    path = "/admin/products",
    request_body = Vec<ProductData>,
    responses(
        (status = OK, body = Vec<ProductId>),
        (status = INTERNAL_SERVER_ERROR)
    ),
    tags = ["admin"]
)]
pub async fn define_products<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Now(now): Now,
    Json(products): Json<Vec<T::ProductData>>,
) -> Result<(StatusCode, Json<Vec<ProductId>>), StatusCode> {
    let ids = T::define_products(&state.market, products.into_iter(), now).await;

    match ids {
        Ok(ids) => Ok((StatusCode::OK, Json(ids))),
        Err(e) => {
            event!(Level::ERROR, error = ?e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
