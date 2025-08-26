use super::Id;
use crate::ApiApplication;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_extra::TypedHeader;
use fts_core::{
    models::ProductRecord,
    ports::{ProductRepository as _, Repository},
};
use headers::{Authorization, authorization::Bearer};
use tracing::{Level, event};

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
/// - `201 Created`: Product created successfully, returns the product record
/// - `401 Unauthorized`: Missing management permissions
/// - `500 Internal Server Error`: Database operation failed
pub(crate) async fn create_product<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(product_data): Json<T::ProductData>,
) -> Result<
    (
        StatusCode,
        Json<ProductRecord<T::Repository, T::ProductData>>,
    ),
    StatusCode,
> {
    // Notice that can_create does not have a &self parameter, while can_read above does.
    // In this case, we expect D::can_create to check if the `auth` corresponds to an admin
    // user.
    if app.can_manage_products(&auth).await {
        let db = app.database();
        let (product_id, as_of) = app.generate_product_id(&product_data);

        let product_record = db
            .create_product(product_id, product_data, as_of)
            .await
            .map_err(|err| {
                event!(Level::ERROR, err = err.to_string());
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        Ok((StatusCode::CREATED, Json(product_record)))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
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
/// - `200 OK`: Product record
/// - `401 Unauthorized`: Missing view permissions
/// - `404 Not Found`: Product does not exist
/// - `500 Internal Server Error`: Database query failed
pub(crate) async fn read_product<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { product_id }): Path<Id<<T::Repository as Repository>::ProductId>>,
) -> Result<Json<ProductRecord<T::Repository, T::ProductData>>, StatusCode> {
    let as_of = app.now();
    let db = app.database();

    if app.can_view_products(&auth).await {
        let product_record = db
            .get_product(product_id, as_of)
            .await
            .map_err(|err| {
                event!(Level::ERROR, err = err.to_string());
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

        Ok(Json(product_record))
    } else {
        Err(StatusCode::UNAUTHORIZED)
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
/// - `201 Created`: List of created child product records, in the same order as the request
/// - `401 Unauthorized`: Missing management permissions
/// - `403 Forbidden`: The product has already been partitioned
/// - `404 Not Found`: Parent product does not exist
/// - `500 Internal Server Error`: Database operation failed
pub(crate) async fn update_product<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { product_id }): Path<Id<<T::Repository as Repository>::ProductId>>,
    Json(children): Json<Vec<PartitionItem<T::ProductData>>>,
) -> Result<
    (
        StatusCode,
        Json<Vec<ProductRecord<T::Repository, T::ProductData>>>,
    ),
    StatusCode,
> {
    let as_of = app.now();
    let db = app.database();

    if app.can_manage_products(&auth).await {
        if children.is_empty() {
            return Err(StatusCode::BAD_REQUEST);
        }

        // compile the specified data and the ids into the appropriate format for the partition function
        let child_data = children
            .into_iter()
            .map(
                |PartitionItem {
                     app_data: data,
                     ratio,
                 }| (app.generate_product_id(&data).0, data, ratio),
            )
            .collect::<Vec<_>>();

        // do the partitioning
        let child_records = db
            .partition_product(product_id.clone(), child_data, as_of)
            .await
            .map_err(|err| {
                event!(Level::ERROR, err = err.to_string());
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;

        if child_records.is_empty() {
            Err(StatusCode::FORBIDDEN)
        } else {
            Ok((StatusCode::CREATED, Json(child_records)))
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Request body item for partitioning a product.
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
pub(crate) struct PartitionItem<D> {
    /// Application-specific data for the child product
    app_data: D,
    /// Weight of this child relative to the parent
    ratio: f64,
}
