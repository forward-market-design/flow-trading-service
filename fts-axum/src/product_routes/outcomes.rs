use super::Id;
use crate::{ApiApplication, config::AxumConfig};

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_extra::TypedHeader;
use fts_core::{
    models::{DateTimeRangeQuery, DateTimeRangeResponse},
    ports::{BatchRepository as _, ProductRepository as _, Repository, Solver},
};
use headers::{Authorization, authorization::Bearer};
use std::sync::Arc;
use tracing::{Level, event};

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
pub(crate) async fn get_product_outcomes<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { product_id }): Path<Id<<T::Repository as Repository>::ProductId>>,
    Extension(config): Extension<Arc<AxumConfig>>,
    Query(query): Query<DateTimeRangeQuery<<T::Repository as Repository>::DateTime>>,
) -> Result<
    Json<
        DateTimeRangeResponse<
            <T::Solver as Solver<
                <T::Repository as Repository>::DemandId,
                <T::Repository as Repository>::PortfolioId,
                <T::Repository as Repository>::ProductId,
            >>::ProductOutcome,
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
