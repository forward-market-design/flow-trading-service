use super::Id;
use crate::ApiApplication;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_extra::TypedHeader;
use fts_core::{
    models::{DemandGroup, PortfolioRecord, ProductGroup},
    ports::{PortfolioRepository as _, Repository},
};
use headers::{Authorization, authorization::Bearer};
use std::hash::Hash;
use tracing::{Level, event};

pub(crate) async fn create_portfolio<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(body): Json<
        CreatePortfolioDto<
            T::PortfolioData,
            <T::Repository as Repository>::DemandId,
            <T::Repository as Repository>::ProductId,
        >,
    >,
) -> Result<
    (
        StatusCode,
        Json<PortfolioRecord<T::Repository, T::PortfolioData>>,
    ),
    StatusCode,
> {
    let as_of = app.now();
    let db = app.database();
    let portfolio_id = app.generate_portfolio_id(&body.app_data);
    let bidder_id = app
        .can_create_bid(&auth)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    db.create_portfolio(
        portfolio_id,
        bidder_id,
        body.app_data,
        body.demand_group,
        body.product_group,
        as_of.clone(),
    )
    .await
    .map(|portfolio| (StatusCode::CREATED, Json(portfolio)))
    .map_err(|err| {
        event!(Level::ERROR, err = err.to_string());
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

/// Retrieve a portfolio's current state.
///
/// Returns the portfolio data including its demand group, product group,
/// and application-specific data. Product groups can be expanded to include
/// any child products created through partitioning.
///
/// # Authorization
///
/// Requires read permission for the portfolio's bidder (`can_read_bid`).
///
/// # Returns
///
/// - `200 OK`: Portfolio data
/// - `401 Unauthorized`: Missing read permissions
/// - `404 Not Found`: Portfolio does not exist
/// - `500 Internal Server Error`: Database query failed
pub(crate) async fn read_portfolio<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { portfolio_id }): Path<Id<<T::Repository as Repository>::PortfolioId>>,
    Query(params): Query<GetPortfolioQuery>,
) -> Result<Json<PortfolioRecord<T::Repository, T::PortfolioData>>, StatusCode> {
    let as_of = app.now();
    let db = app.database();
    let portfolio = if params.expand {
        db.get_portfolio_with_expanded_products(portfolio_id, as_of)
            .await
    } else {
        db.get_portfolio(portfolio_id, as_of).await
    }
    .map_err(|err| {
        event!(Level::ERROR, err = err.to_string());
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    if !app.can_read_bid(&auth, portfolio.bidder_id.clone()).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(portfolio))
}

/// Update a portfolio's demand and/or product associations.
///
/// Either or both groups can be updated by providing non-None values.
/// Providing None for a group leaves it unchanged.
///
/// # Authorization
///
/// Requires update permission for the portfolio's bidder (`can_update_bid`).
///
/// # Returns
///
/// - `200 OK`: Portfolio updated successfully
/// - `401 Unauthorized`: Missing update permissions
/// - `404 Not Found`: Portfolio does not exist
/// - `500 Internal Server Error`: Database operation failed
pub(crate) async fn update_portfolio<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { portfolio_id }): Path<Id<<T::Repository as Repository>::PortfolioId>>,
    Json(body): Json<
        UpdatePortfolioDto<
            <T::Repository as Repository>::DemandId,
            <T::Repository as Repository>::ProductId,
        >,
    >,
) -> Result<Json<PortfolioRecord<T::Repository, T::PortfolioData>>, StatusCode> {
    let as_of = app.now();
    let db = app.database();
    let bidder_id = db
        .get_portfolio_bidder_id(portfolio_id.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    if !app.can_update_bid(&auth, bidder_id).await {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let updated = match (body.demand_group, body.product_group) {
        (Some(demand_group), Some(product_group)) => {
            db.update_portfolio_groups(portfolio_id, demand_group, product_group, as_of)
                .await
        }
        (Some(demand_group), None) => {
            db.update_portfolio_demand_group(portfolio_id, demand_group, as_of)
                .await
        }
        (None, Some(product_group)) => {
            db.update_portfolio_product_group(portfolio_id, product_group, as_of)
                .await
        }
        (None, None) => db.get_portfolio(portfolio_id, as_of).await,
    }
    .map_err(|err| {
        event!(Level::ERROR, err = err.to_string());
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or_else(|| {
        event!(
            Level::ERROR,
            err = "failed to update portfolio after successful read"
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(updated))
}

/// Delete a portfolio by clearing both its demand and product groups.
///
/// This doesn't remove the portfolio from the database but deactivates it
/// by setting both groups to None. The portfolio's history is preserved.
///
/// # Authorization
///
/// Requires update permission for the portfolio's bidder (`can_update_bid`).
///
/// # Returns
///
/// - `200 OK`: Portfolio deleted successfully
/// - `401 Unauthorized`: Missing update permissions
/// - `404 Not Found`: Portfolio does not exist
/// - `500 Internal Server Error`: Database operation failed
pub(crate) async fn delete_portfolio<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { portfolio_id }): Path<Id<<T::Repository as Repository>::PortfolioId>>,
) -> Result<Json<PortfolioRecord<T::Repository, T::PortfolioData>>, StatusCode> {
    let as_of = app.now();
    let db = app.database();
    let bidder_id = db
        .get_portfolio_bidder_id(portfolio_id.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    if !app.can_update_bid(&auth, bidder_id).await {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let deleted = db
        .update_portfolio_groups(portfolio_id, Default::default(), Default::default(), as_of)
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            event!(
                Level::ERROR,
                err = "failed to delete portfolio after successful read"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(deleted))
}

/// Request body for creating a new portfolio.
#[derive(schemars::JsonSchema, serde::Deserialize)]
#[schemars(inline)]
pub(crate) struct CreatePortfolioDto<PortfolioData, DemandId: Eq + Hash, ProductId: Eq + Hash> {
    /// Application-specific data to associate with the portfolio
    app_data: PortfolioData,
    /// Initial demand weights
    demand_group: DemandGroup<DemandId>,
    /// Initial product weights
    product_group: ProductGroup<ProductId>,
}

/// Request body for updating a portfolio's groups.
#[derive(schemars::JsonSchema, serde::Deserialize)]
#[schemars(inline)]
pub(crate) struct UpdatePortfolioDto<DemandId: Eq + Hash, ProductId: Eq + Hash> {
    /// New demand group weights (None to keep existing)
    demand_group: Option<DemandGroup<DemandId>>,
    /// New product group weights (None to keep existing)
    product_group: Option<ProductGroup<ProductId>>,
}

#[derive(schemars::JsonSchema, serde::Deserialize)]
#[schemars(inline)]
pub(crate) struct GetPortfolioQuery {
    #[serde(default)]
    expand: bool,
}
