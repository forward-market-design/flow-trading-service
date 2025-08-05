use super::Id;
use crate::{ApiApplication, config::AxumConfig};

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_extra::TypedHeader;
use fts_core::{
    models::{DateTimeRangeQuery, DateTimeRangeResponse, DemandGroup, Basis},
    ports::{PortfolioRepository as _, Repository},
};
use headers::{Authorization, authorization::Bearer};
use std::sync::Arc;
use tracing::{Level, event};

/// Retrieve the historical changes to a portfolio's demand group.
///
/// Returns a paginated list of demand group changes over time, showing
/// how the portfolio's demand associations have evolved.
///
/// # Authorization
///
/// Requires read permission for the portfolio's bidder (`can_read_bid`).
///
/// # Returns
///
/// - `200 OK`: Paginated demand group history records
/// - `401 Unauthorized`: Missing read permissions
/// - `404 Not Found`: Portfolio does not exist
/// - `500 Internal Server Error`: Database query failed
pub(crate) async fn get_portfolio_demand_history<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { portfolio_id }): Path<Id<<T::Repository as Repository>::PortfolioId>>,
    Extension(config): Extension<Arc<AxumConfig>>,
    Query(query): Query<DateTimeRangeQuery<<T::Repository as Repository>::DateTime>>,
) -> Result<
    Json<
        DateTimeRangeResponse<
            DemandGroup<<T::Repository as Repository>::DemandId>,
            <T::Repository as Repository>::DateTime,
        >,
    >,
    StatusCode,
> {
    let db = app.database();

    // Check if the user is authorized to read the portfolio history
    let bidder_id = db
        .get_portfolio_bidder_id(portfolio_id.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    if !app.can_read_bid(&auth, bidder_id).await {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let history = db
        .get_portfolio_demand_history(portfolio_id, query, config.page_limit)
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(history))
}

/// Retrieve the historical changes to a portfolio's product group.
///
/// Returns a paginated list of product group changes over time, showing
/// how the portfolio's product associations have evolved.
///
/// # Authorization
///
/// Requires read permission for the portfolio's bidder (`can_read_bid`).
///
/// # Returns
///
/// - `200 OK`: Paginated product group history records
/// - `401 Unauthorized`: Missing read permissions
/// - `404 Not Found`: Portfolio does not exist
/// - `500 Internal Server Error`: Database query failed
pub(crate) async fn get_portfolio_product_history<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { portfolio_id }): Path<Id<<T::Repository as Repository>::PortfolioId>>,
    Extension(config): Extension<Arc<AxumConfig>>,
    Query(query): Query<DateTimeRangeQuery<<T::Repository as Repository>::DateTime>>,
) -> Result<
    Json<
        DateTimeRangeResponse<
            Basis<<T::Repository as Repository>::ProductId>,
            <T::Repository as Repository>::DateTime,
        >,
    >,
    StatusCode,
> {
    let db = app.database();

    // Check if the user is authorized to read the portfolio history
    let bidder_id = db
        .get_portfolio_bidder_id(portfolio_id.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    if !app.can_read_bid(&auth, bidder_id).await {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let history = db
        .get_portfolio_product_history(portfolio_id, query, config.page_limit)
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(history))
}
