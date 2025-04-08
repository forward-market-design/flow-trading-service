use crate::AppState;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use fts_core::{
    models::{
        AuctionOutcome, DateTimeRangeQuery, DateTimeRangeResponse, ProductData, ProductId,
        ProductQuery, ProductQueryResponse, ProductRecord,
    },
    ports::{MarketRepository, ProductRepository},
};
use tracing::{Level, event};

pub fn router<T: MarketRepository>() -> Router<AppState<T>> {
    Router::new()
        // Query the product directory
        .route("/", routing::get(list_products))
        // Get all data for a certain product
        .route("/{product_id}", routing::get(get_product))
        // View the results associated to a product
        .route("/{product_id}/outcomes", routing::get(product_outcomes))
}

#[utoipa::path(
    get,
    path = "/v0/products",
    responses(
        (status = OK, body = ProductQueryResponse<ProductRecord<String>, ProductQuery<String>>),
        (status = INTERNAL_SERVER_ERROR)
    ),
    params(
        ("example_query" = ProductQuery<String>, Query)
    ),
    tags = ["products"]
)]
async fn list_products<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Query(query): Query<T::ProductQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let results = T::query_products(&state.market, query, 100).await;
    match results {
        Ok(results) => Ok((StatusCode::OK, Json(results))),
        Err(e) => {
            event!(Level::ERROR, error = ?e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[utoipa::path(
    get,
    path = "/v0/products/{product_id}",
    responses(
        (status = OK, body = ProductRecord<ProductData<String>>),
        (status = NOT_FOUND),
        (status = INTERNAL_SERVER_ERROR)
    ),
    params(
        ("product_id" = ProductId, description = "Unique identifier of the product")
    ),
    tags = ["products"]
)]
/// Retrieves the product specified by the route
async fn get_product<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Path(product_id): Path<ProductId>,
) -> Result<impl IntoResponse, StatusCode> {
    let data = T::view_product(&state.market, product_id).await;

    match data {
        Ok(Some(data)) => Ok((StatusCode::OK, Json(data))),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            event!(Level::ERROR, error = ?e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

type ProductOutcomeBody = DateTimeRangeResponse<AuctionOutcome<()>>;

#[utoipa::path(
    get,
    path = "/v0/products/{product_id}/outcomes",
    responses(
        (status = OK, body = ProductOutcomeBody),
        (status = INTERNAL_SERVER_ERROR)
    ),
    params(
        ("product_id" = ProductId, description = "Unique identifier of the product"),
        DateTimeRangeQuery
    ),
    tags = ["products", "outcomes"]
)]
/// Retrieves any outcomes associated to the product
async fn product_outcomes<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Path(product_id): Path<ProductId>,
    Query(query): Query<DateTimeRangeQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let data = ProductRepository::get_outcomes(&state.market, product_id, query, 48).await;

    match data {
        Ok(data) => Ok((StatusCode::OK, Json(data))),
        Err(e) => {
            event!(Level::ERROR, error = ?e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
