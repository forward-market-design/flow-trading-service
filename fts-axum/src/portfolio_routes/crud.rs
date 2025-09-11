use super::Id;
use crate::{ApiApplication, config::AxumConfig};

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_extra::TypedHeader;
use fts_core::{
    models::{Basis, PortfolioRecord, Weights},
    ports::{BatchRepository as _, PortfolioRepository as _, Repository},
};
use headers::{Authorization, authorization::Bearer};
use std::{hash::Hash, sync::Arc};
use tracing::{Level, event};

pub(crate) async fn create_portfolio<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Extension(config): Extension<Arc<AxumConfig>>,
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
    let db = app.database();
    let (portfolio_id, as_of) = app.generate_portfolio_id(&body.app_data);
    let bidder_id = app
        .can_create_bid(&auth)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let created = db
        .create_portfolio(
            portfolio_id,
            bidder_id,
            body.app_data,
            body.demand,
            body.basis,
            as_of.clone(),
        )
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if let Some(config) = config.auto_solve.clone() {
        tokio::spawn(async move {
            let db = app.database();
            let result = db
                .run_batch(as_of, config, app.solver(), Default::default())
                .await;

            match result {
                Err(err) => {
                    event!(Level::ERROR, err = err.to_string());
                }
                Ok(Err(err)) => {
                    event!(Level::ERROR, err = err.to_string());
                }
                _ => {}
            };
        });
    };

    Ok((StatusCode::CREATED, Json(created)))
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
    Extension(config): Extension<Arc<AxumConfig>>,
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

    let updated = match (body.demand, body.basis) {
        (Some(demand), Some(basis)) => {
            db.update_portfolio(portfolio_id, demand, basis, as_of.clone())
                .await
        }
        (Some(demand), None) => {
            db.update_portfolio_demand(portfolio_id, demand, as_of.clone())
                .await
        }
        (None, Some(basis)) => {
            db.update_portfolio_basis(portfolio_id, basis, as_of.clone())
                .await
        }
        (None, None) => db.get_portfolio(portfolio_id, as_of.clone()).await,
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

    if let Some(config) = config.auto_solve.clone() {
        tokio::spawn(async move {
            let db = app.database();
            let result = db
                .run_batch(as_of, config, app.solver(), Default::default())
                .await;

            match result {
                Err(err) => {
                    event!(Level::ERROR, err = err.to_string());
                }
                Ok(Err(err)) => {
                    event!(Level::ERROR, err = err.to_string());
                }
                _ => {}
            };
        });
    };

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
    Extension(config): Extension<Arc<AxumConfig>>,
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
        .update_portfolio(
            portfolio_id,
            Default::default(),
            Default::default(),
            as_of.clone(),
        )
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

    if let Some(config) = config.auto_solve.clone() {
        tokio::spawn(async move {
            let db = app.database();
            let result = db
                .run_batch(as_of, config, app.solver(), Default::default())
                .await;

            match result {
                Err(err) => {
                    event!(Level::ERROR, err = err.to_string());
                }
                Ok(Err(err)) => {
                    event!(Level::ERROR, err = err.to_string());
                }
                _ => {}
            };
        });
    };

    Ok(Json(deleted))
}

/// Request body for creating a new portfolio.
#[derive(schemars::JsonSchema, serde::Deserialize)]
#[schemars(inline)]
pub(crate) struct CreatePortfolioDto<PortfolioData, DemandId: Eq + Hash, ProductId: Eq + Hash> {
    /// Application-specific data to associate with the portfolio
    app_data: PortfolioData,
    /// Initial demand weights
    demand: Weights<DemandId>,
    /// Initial product weights
    basis: Basis<ProductId>,
}

/// Request body for updating a portfolio's groups.
#[derive(schemars::JsonSchema, serde::Deserialize)]
#[schemars(inline)]
pub(crate) struct UpdatePortfolioDto<DemandId: Eq + Hash, ProductId: Eq + Hash> {
    /// New demand group weights (None to keep existing)
    demand: Option<Weights<DemandId>>,
    /// New product group weights (None to keep existing)
    basis: Option<Basis<ProductId>>,
}

#[derive(schemars::JsonSchema, serde::Deserialize)]
#[schemars(inline)]
pub(crate) struct GetPortfolioQuery {
    #[serde(default)]
    expand: bool,
}
