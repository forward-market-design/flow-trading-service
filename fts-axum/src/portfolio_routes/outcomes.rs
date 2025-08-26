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
    ports::{BatchRepository, PortfolioRepository as _, Repository, Solver},
};
use headers::{Authorization, authorization::Bearer};
use std::sync::Arc;
use tracing::{Level, event};

/// Retrieve batch auction outcomes for a portfolio.
///
/// Returns the historical allocations computed by the solver for this
/// portfolio across multiple batch auctions.
///
/// # Authorization
///
/// Requires read permission for the portfolio's bidder (`can_read_bid`).
///
/// # Returns
///
/// - `200 OK`: Paginated outcome records
/// - `401 Unauthorized`: Missing read permissions
/// - `404 Not Found`: Portfolio does not exist
/// - `500 Internal Server Error`: Database query failed
pub(crate) async fn get_portfolio_outcomes<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { portfolio_id }): Path<Id<<T::Repository as Repository>::PortfolioId>>,
    Extension(config): Extension<Arc<AxumConfig>>,
    Query(query): Query<DateTimeRangeQuery<<T::Repository as Repository>::DateTime>>,
) -> Result<
    Json<
        DateTimeRangeResponse<
            <T::Solver as Solver<
                <T::Repository as Repository>::DemandId,
                <T::Repository as Repository>::PortfolioId,
                <T::Repository as Repository>::ProductId,
            >>::PortfolioOutcome,
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

    let outcomes = db
        .get_portfolio_outcomes(portfolio_id, query, config.page_limit)
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(outcomes))
}
