//! REST API endpoints for portfolio operations.
//!
//! This module provides CRUD operations for portfolios, which aggregate demands
//! and associate them with tradeable products. Portfolios enable complex trading
//! strategies by allowing weighted combinations of demands and products.

use crate::{ApiApplication, config::AxumConfig};
use aide::{
    axum::{
        ApiRouter,
        routing::{get, get_with},
    },
    transform::TransformOperation,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_extra::TypedHeader;
use fts_core::{
    models::{
        DateTimeRangeQuery, DateTimeRangeResponse, Map, OutcomeRecord, PortfolioRecord, ValueRecord,
    },
    ports::{BatchRepository, PortfolioRepository as _, Repository, Solver},
};
use headers::{Authorization, authorization::Bearer};
use std::{hash::Hash, sync::Arc};
use tracing::{Level, event};

/// Path parameter for portfolio-specific endpoints.
#[derive(serde::Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
struct Id<T> {
    /// The unique identifier of the portfolio
    portfolio_id: T,
}

/// Creates a router with portfolio-related endpoints.
pub fn router<T: ApiApplication>() -> ApiRouter<T> {
    ApiRouter::new()
        .api_route_with(
            "/",
            get_with(query_portfolios::<T>, query_portfolios_docs)
                .post_with(create_portfolio::<T>, create_portfolio_docs),
            |route| route.security_requirement("jwt").tag("portfolio"),
        )
        .api_route_with(
            "/{portfolio_id}",
            get(get_portfolio::<T>)
                .patch(update_portfolio::<T>)
                .delete(delete_portfolio::<T>),
            |route| route.security_requirement("jwt").tag("portfolio"),
        )
        .api_route_with(
            "/{portfolio_id}/demand-history",
            get(get_portfolio_demand_history::<T>),
            |route| {
                route
                    .security_requirement("jwt")
                    .tag("portfolio")
                    .tag("history")
            },
        )
        .api_route_with(
            "/{portfolio_id}/product-history",
            get(get_portfolio_product_history::<T>),
            |route| {
                route
                    .security_requirement("jwt")
                    .tag("portfolio")
                    .tag("history")
            },
        )
        .api_route_with(
            "/{portfolio_id}/outcomes",
            get(get_portfolio_outcomes::<T>),
            |route| {
                route
                    .security_requirement("jwt")
                    .tag("portfolio")
                    .tag("outcome")
            },
        )
}

fn query_portfolios_docs(op: TransformOperation) -> TransformOperation<'_> {
    op.summary("List portfolios")
        .description(
            r#"
            Query all portfolios for bidders the requester is authorized to view.
            Returns only portfolios with non-empty demand or product groups.

            Requires `can_query_bid` permission.
            "#,
        )
        //.response_with::<200, _, _>(|res| res.description("List of portfolio IDs"))
        .response_with::<401, String, _>(|res| res.description("Unauthorized"))
        .response_with::<500, String, _>(|res| res.description("Database query failed"))
}

async fn query_portfolios<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Vec<<T::Repository as Repository>::PortfolioId>>, (StatusCode, String)> {
    let as_of = app.now();
    let db = app.database();
    let bidder_ids = app.can_query_bid(&auth).await;

    if bidder_ids.is_empty() {
        Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()))
    } else {
        Ok(Json(db.query_portfolio(&bidder_ids, as_of).await.map_err(
            |err| {
                event!(Level::ERROR, err = err.to_string());
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "failed to query demand".to_string(),
                )
            },
        )?))
    }
}

fn create_portfolio_docs(op: TransformOperation) -> TransformOperation<'_> {
    op.summary("Create Portfolio")
        .description(
            r#"
        Create a new portfolio with initial demand and product associations.

        Requires `can_create_bid` permission. The portfolio will be
        associated with the bidder determined by the authorization context. 
        "#,
        )
        // FIXME: The 201 is not automatically generated, but manually documenting it here is... not nice.
        .response_with::<401, String, _>(|res| res.description("Missing create permissions"))
        .response_with::<500, String, _>(|res| res.description("Database operation failed"))
}

async fn create_portfolio<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(body): Json<
        CreatePortfolioRequestBody<
            T::PortfolioData,
            <T::Repository as Repository>::DemandId,
            <T::Repository as Repository>::ProductId,
        >,
    >,
) -> Result<
    (
        StatusCode,
        Json<
            CreatePortfolioResponseBody<
                <T::Repository as Repository>::DateTime,
                <T::Repository as Repository>::PortfolioId,
            >,
        >,
    ),
    (StatusCode, String),
> {
    let as_of = app.now();
    let db = app.database();
    let portfolio_id = app.generate_portfolio_id(&body.app_data);
    let bidder_id = app
        .can_create_bid(&auth)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "not authorized".to_string()))?;

    db.create_portfolio(
        portfolio_id.clone(),
        bidder_id,
        body.app_data,
        body.demand_group,
        body.product_group,
        as_of.clone(),
    )
    .await
    .map(|_| {
        (
            StatusCode::CREATED,
            Json(CreatePortfolioResponseBody {
                as_of,
                portfolio_id,
            }),
        )
    })
    .map_err(|err| {
        event!(Level::ERROR, err = err.to_string());
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to create portfolio".to_string(),
        )
    })
}

/// Retrieve a portfolio's current state.
///
/// Returns the portfolio data including its demand group, product group,
/// and application-specific data. Product groups are expanded to include
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
async fn get_portfolio<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { portfolio_id }): Path<Id<<T::Repository as Repository>::PortfolioId>>,
) -> Result<
    Json<
        PortfolioRecord<
            <T::Repository as Repository>::DateTime,
            <T::Repository as Repository>::BidderId,
            <T::Repository as Repository>::PortfolioId,
            <T::Repository as Repository>::DemandId,
            <T::Repository as Repository>::ProductId,
            T::PortfolioData,
        >,
    >,
    (StatusCode, String),
> {
    let as_of = app.now();
    let db = app.database();
    let portfolio = db
        .get_portfolio(portfolio_id.clone(), as_of)
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to get portfolio".to_string(),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, "portfolio not found".to_string()))?;

    if !app.can_read_bid(&auth, portfolio.bidder_id.clone()).await {
        return Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()));
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
async fn update_portfolio<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { portfolio_id }): Path<Id<<T::Repository as Repository>::PortfolioId>>,
    Json(body): Json<
        UpdatePortfolioDto<
            <T::Repository as Repository>::DemandId,
            <T::Repository as Repository>::ProductId,
        >,
    >,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let as_of = app.now();
    let db = app.database();
    let bidder_id = db
        .get_portfolio_bidder_id(portfolio_id.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get portfolio {}", portfolio_id),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("unknown portfolio {}", portfolio_id),
        ))?;

    if !app.can_update_bid(&auth, bidder_id).await {
        return Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()));
    }

    let updated = db
        .update_portfolio(
            portfolio_id.clone(),
            body.demand_group,
            body.product_group,
            as_of.clone(),
        )
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to update portfolio {}", portfolio_id),
            )
        })?;

    if updated {
        Ok((StatusCode::OK, format!("{}", as_of)))
    } else {
        // Since we got the portfolio for the initial permission check,
        // `updated` should always be true unless something weird happened.
        event!(
            Level::ERROR,
            err = "failed to update portfolio after successful read"
        );
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to update portfolio {}", portfolio_id),
        ))
    }
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
async fn delete_portfolio<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { portfolio_id }): Path<Id<<T::Repository as Repository>::PortfolioId>>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let as_of = app.now();
    let db = app.database();
    let bidder_id = db
        .get_portfolio_bidder_id(portfolio_id.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get portfolio {}", portfolio_id),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("unknown portfolio {}", portfolio_id),
        ))?;

    if !app.can_update_bid(&auth, bidder_id).await {
        return Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()));
    }

    let deleted = db
        .update_portfolio(
            portfolio_id.clone(),
            Some(Default::default()),
            Some(Default::default()),
            as_of.clone(),
        )
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to delete portfolio {}", portfolio_id),
            )
        })?;

    if deleted {
        Ok((StatusCode::OK, format!("{}", as_of)))
    } else {
        // Since we got the portfolio for the initial permission check,
        // `updated` should always be true unless something weird happened.
        event!(
            Level::ERROR,
            err = "failed to delete portfolio after successful read"
        );
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to delete portfolio {}", portfolio_id),
        ))
    }
}

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
async fn get_portfolio_demand_history<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { portfolio_id }): Path<Id<<T::Repository as Repository>::PortfolioId>>,
    Extension(config): Extension<Arc<AxumConfig>>,
    Query(query): Query<DateTimeRangeQuery<<T::Repository as Repository>::DateTime>>,
) -> Result<
    Json<
        DateTimeRangeResponse<
            ValueRecord<
                <T::Repository as Repository>::DateTime,
                Map<<T::Repository as Repository>::DemandId, f64>,
            >,
            <T::Repository as Repository>::DateTime,
        >,
    >,
    (StatusCode, String),
> {
    let db = app.database();

    // Check if the user is authorized to read the portfolio history
    let bidder_id = db
        .get_portfolio_bidder_id(portfolio_id.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get portfolio {}", portfolio_id),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("unknown portfolio {}", portfolio_id),
        ))?;

    if !app.can_read_bid(&auth, bidder_id).await {
        return Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()));
    }

    let history = db
        .get_portfolio_demand_history(portfolio_id.clone(), query, config.page_limit)
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get portfolio demand history {}", portfolio_id),
            )
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
async fn get_portfolio_product_history<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { portfolio_id }): Path<Id<<T::Repository as Repository>::PortfolioId>>,
    Extension(config): Extension<Arc<AxumConfig>>,
    Query(query): Query<DateTimeRangeQuery<<T::Repository as Repository>::DateTime>>,
) -> Result<
    Json<
        DateTimeRangeResponse<
            ValueRecord<
                <T::Repository as Repository>::DateTime,
                Map<<T::Repository as Repository>::ProductId, f64>,
            >,
            <T::Repository as Repository>::DateTime,
        >,
    >,
    (StatusCode, String),
> {
    let db = app.database();

    // Check if the user is authorized to read the portfolio history
    let bidder_id = db
        .get_portfolio_bidder_id(portfolio_id.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get portfolio {}", portfolio_id),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("unknown portfolio {}", portfolio_id),
        ))?;

    if !app.can_read_bid(&auth, bidder_id).await {
        return Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()));
    }

    let history = db
        .get_portfolio_product_history(portfolio_id.clone(), query, config.page_limit)
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get portfolio product history {}", portfolio_id),
            )
        })?;

    Ok(Json(history))
}

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
async fn get_portfolio_outcomes<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { portfolio_id }): Path<Id<<T::Repository as Repository>::PortfolioId>>,
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
                >>::PortfolioOutcome,
            >,
            <T::Repository as Repository>::DateTime,
        >,
    >,
    (StatusCode, String),
> {
    let db = app.database();

    // Check if the user is authorized to read the portfolio history
    let bidder_id = db
        .get_portfolio_bidder_id(portfolio_id.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get portfolio {}", portfolio_id),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("unknown portfolio {}", portfolio_id),
        ))?;

    if !app.can_read_bid(&auth, bidder_id).await {
        return Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()));
    }

    let outcomes = db
        .get_portfolio_outcomes(portfolio_id.clone(), query, config.page_limit)
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get portfolio outcomes {}", portfolio_id),
            )
        })?;

    Ok(Json(outcomes))
}

/// Request body for updating a portfolio's groups.
#[derive(schemars::JsonSchema, serde::Deserialize)]
#[schemars(inline)]
struct UpdatePortfolioDto<DemandId: Eq + Hash, ProductId: Eq + Hash> {
    /// New demand group weights (None to keep existing)
    demand_group: Option<Map<DemandId>>,
    /// New product group weights (None to keep existing)
    product_group: Option<Map<ProductId>>,
}

/// Request body for creating a new portfolio.
#[derive(schemars::JsonSchema, serde::Deserialize)]
#[schemars(inline)]
struct CreatePortfolioRequestBody<PortfolioData, DemandId: Eq + Hash, ProductId: Eq + Hash> {
    /// Application-specific data to associate with the portfolio
    app_data: PortfolioData,
    /// Initial demand weights
    demand_group: Map<DemandId>,
    /// Initial product weights
    product_group: Map<ProductId>,
}

/// Response body for creating a new portfolio
#[derive(serde::Serialize, schemars::JsonSchema)]
#[schemars(inline)]
struct CreatePortfolioResponseBody<T, U> {
    /// The effective timestamp of the portfolio
    as_of: T,
    /// The system-generated id of the portfolio
    portfolio_id: U,
}
