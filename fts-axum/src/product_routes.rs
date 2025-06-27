//! REST API endpoints for product operations.
//!
//! This module provides operations for managing the product hierarchy, including
//! creating root products and partitioning them into child products. Products
//! represent the tradeable assets in the flow trading system.

use std::sync::Arc;

use crate::{ApiApplication, config::AxumConfig};
use aide::axum::{
    ApiRouter,
    routing::{get, post},
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_extra::TypedHeader;
use fts_core::{
    models::{DateTimeRangeQuery, DateTimeRangeResponse, OutcomeRecord},
    ports::{BatchRepository as _, ProductRepository as _, Repository, Solver},
};
use headers::{Authorization, authorization::Bearer};
use tracing::{Level, event};

/// Path parameter for product-specific endpoints.
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct Id<T> {
    /// The unique identifier of the product
    product_id: T,
}

/// Creates a router with product-related endpoints.
pub fn router<T: ApiApplication>() -> ApiRouter<T> {
    ApiRouter::new()
        // TODO: figure out approach to querying!
        .api_route("/", post(create_product::<T>))
        .api_route("/{product_id}", get(get_product::<T>))
        .api_route("/{product_id}/partition", post(partition_product::<T>))
        .api_route("/{product_id}/outcomes", get(get_product_outcomes::<T>))
}

/// Retrieve a product's data.
///
/// Returns the application-specific data associated with the product.
///
/// # Authorization
///
/// Requires `can_view_products` permission.
///
/// # Returns
///
/// - `200 OK`: Product data
/// - `401 Unauthorized`: Missing view permissions
/// - `404 Not Found`: Product does not exist
/// - `500 Internal Server Error`: Database query failed
async fn get_product<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { product_id }): Path<Id<<T::Repository as Repository>::ProductId>>,
) -> Result<Json<T::ProductData>, (StatusCode, String)> {
    let as_of = app.now();
    let db = app.database();

    if app.can_view_products(&auth).await {
        let product_data = db
            .get_product(product_id.clone(), as_of)
            .await
            .map_err(|err| {
                event!(Level::ERROR, err = err.to_string());
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to get product {}", product_id),
                )
            })?
            .ok_or((
                StatusCode::NOT_FOUND,
                format!("unknown product {}", product_id),
            ))?;
        Ok(Json(product_data))
    } else {
        Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()))
    }
}

/// Create a new root product.
///
/// Creates a product with no parent in the product hierarchy.
///
/// # Request Body
///
/// Application-specific product data. The product ID will be generated
/// based on this data.
///
/// # Authorization
///
/// Requires `can_manage_products` permission.
///
/// # Returns
///
/// - `201 Created`: Product created successfully, returns the product ID
/// - `401 Unauthorized`: Missing management permissions
/// - `500 Internal Server Error`: Database operation failed
async fn create_product<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(product_data): Json<T::ProductData>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // Notice that can_create does not have a &self parameter, while can_read above does.
    // In this case, we expect D::can_create to check if the `auth` corresponds to an admin
    // user.
    if app.can_manage_products(&auth).await {
        let as_of = app.now();
        let db = app.database();
        let product_id = app.generate_product_id(&product_data);

        db.create_product(product_id.clone(), product_data, as_of)
            .await
            .map(|_| (StatusCode::CREATED, format!("{}", product_id)))
            .map_err(|err| {
                event!(Level::ERROR, err = err.to_string());
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to create product".to_string(),
                )
            })
    } else {
        Ok((StatusCode::UNAUTHORIZED, "not authorized".to_string()))
    }
}

/// Partition a product into weighted child products.
///
/// Creates new child products that represent portions of the parent product.
/// The weights determine how allocations to the parent are distributed to
/// the children. This operation is irreversible.
///
/// # Authorization
///
/// Requires `can_manage_products` permission.
///
/// # Returns
///
/// - `201 Created`: List of created child product IDs
/// - `401 Unauthorized`: Missing management permissions
/// - `404 Not Found`: Parent product does not exist
/// - `500 Internal Server Error`: Database operation failed
async fn partition_product<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { product_id }): Path<Id<<T::Repository as Repository>::ProductId>>,
    Json(children): Json<Vec<PartitionItem<T::ProductData>>>,
) -> Result<
    (
        StatusCode,
        Json<Vec<<T::Repository as Repository>::ProductId>>,
    ),
    (StatusCode, String),
> {
    let as_of = app.now();
    let db = app.database();

    if app.can_manage_products(&auth).await {
        // First we get the existing data.
        let _product_data = db
            .get_product(product_id.clone(), as_of.clone())
            .await
            .map_err(|err| {
                event!(Level::ERROR, err = err.to_string());
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to get product {}", product_id),
                )
            })?
            .ok_or((
                StatusCode::NOT_FOUND,
                format!("unknown product {}", product_id),
            ))?;

        // compile the specified data and the ids into the appropriate format for the partition function
        let child_data = children
            .into_iter()
            .map(|PartitionItem { data, ratio }| (app.generate_product_id(&data), data, ratio))
            .collect::<Vec<_>>();

        let ids = child_data.iter().map(|(id, _, _)| id.clone()).collect();

        // do the partitioning
        db.partition_product(product_id.clone(), child_data, as_of)
            .await
            .map_err(|err| {
                event!(Level::ERROR, err = err.to_string());
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to partition {}", product_id),
                )
            })?;

        Ok((StatusCode::CREATED, Json(ids)))
    } else {
        Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()))
    }
}

/// Retrieve batch auction outcomes for a product.
///
/// Returns the historical clearing prices computed by the solver for this
/// product across multiple batch auctions.
///
/// # Authorization
///
/// Requires `can_view_products` permission.
///
/// # Returns
///
/// - `200 OK`: Paginated outcome records
/// - `401 Unauthorized`: Missing view permissions
/// - `500 Internal Server Error`: Database query failed
async fn get_product_outcomes<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { product_id }): Path<Id<<T::Repository as Repository>::ProductId>>,
    Extension(config): Extension<Arc<AxumConfig>>,
    Query(query): Query<DateTimeRangeQuery<<T::Repository as Repository>::DateTime>>,
) -> Result<
    Json<
        DateTimeRangeResponse<
            OutcomeRecord<
                <T::Repository as Repository>::DateTime,
                <T::Solver as Solver<
                    <T::Repository as Repository>::DemandId,
                    <T::Repository as Repository>::PortfolioId,
                    <T::Repository as Repository>::ProductId,
                >>::ProductOutcome,
            >,
            <T::Repository as Repository>::DateTime,
        >,
    >,
    (StatusCode, String),
> {
    let as_of = app.now();
    let db = app.database();

    if !app.can_view_products(&auth).await {
        return Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()));
    }

    // First we get the existing data.
    let _product_data = db
        .get_product(product_id.clone(), as_of)
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get product {}", product_id),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("unknown product {}", product_id),
        ))?;

    let outcomes = db
        .get_product_outcomes(product_id.clone(), query, config.page_limit)
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get product outcomes {}", product_id),
            )
        })?;

    Ok(Json(outcomes))
}

/// Request body item for partitioning a product.
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct PartitionItem<D> {
    /// Application-specific data for the child product
    data: D,
    /// Weight of this child relative to the parent
    ratio: f64,
}
