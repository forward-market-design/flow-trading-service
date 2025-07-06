//! REST API endpoints for demand curve operations.
//!
//! This module provides CRUD operations for demand curves, which represent
//! bidders' pricing preferences in the flow trading system. Demands can be
//! created, updated, deleted, and queried, with full history tracking.

use std::sync::Arc;

use crate::{ApiApplication, config};
use aide::axum::{ApiRouter, routing::get};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use axum_extra::TypedHeader;
use fts_core::{
    models::{DateTimeRangeQuery, DateTimeRangeResponse, DemandCurve, DemandRecord},
    ports::{DemandRepository as _, Repository},
};
use headers::{Authorization, authorization::Bearer};
use tracing::{Level, event};

/// Creates a router with demand-related endpoints.
pub fn router<T: ApiApplication>() -> ApiRouter<T> {
    ApiRouter::new()
        .api_route_with(
            "/",
            get(query_demands::<T>).post(create_demand::<T>),
            |route| route.security_requirement("jwt").tag("demand"),
        )
        .api_route_with(
            "/{demand_id}",
            get(get_demand::<T>)
                .put(update_demand::<T>)
                .delete(delete_demand::<T>),
            |route| route.security_requirement("jwt").tag("demand"),
        )
        .api_route_with(
            "/{demand_id}/curve-history",
            get(get_demand_curve_history::<T>),
            |route| {
                route
                    .security_requirement("jwt")
                    .tag("demand")
                    .tag("history")
            },
        )
}

/// Path parameter for demand-specific endpoints.
#[derive(serde::Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
struct Id<T> {
    /// The unique identifier of the demand
    demand_id: T,
}

/// Query all demands for bidders the requester is authorized to view.
///
/// # Authorization
///
/// Returns demands only for bidders that the context has query access to
/// (`can_query_bid` permission).
///
/// # Returns
///
/// - `200 OK`: List of demand IDs
/// - `401 Unauthorized`: No query permissions for any bidder
/// - `500 Internal Server Error`: Database query failed
async fn query_demands<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Vec<DemandRecord<T::Repository, T::DemandData>>>, (StatusCode, String)> {
    let db = app.database();
    let bidder_ids = app.can_query_bid(&auth).await;

    if bidder_ids.is_empty() {
        Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()))
    } else {
        Ok(Json(db.query_demand(&bidder_ids).await.map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to query demand".to_string(),
            )
        })?))
    }
}

/// Create a new demand with optional initial curve data.
///
/// # Authorization
///
/// Requires create permission (`can_create_bid`). The demand will be
/// associated with the bidder determined by the authorization context.
///
/// # Returns
///
/// - `201 Created`: Demand created successfully, returns the demand ID
/// - `401 Unauthorized`: Missing create permissions
/// - `500 Internal Server Error`: Database operation failed
async fn create_demand<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(body): Json<CreateDemandRequestBody<T::DemandData>>,
) -> Result<
    (
        StatusCode,
        Json<
            CreateDemandResponseBody<
                <T::Repository as Repository>::DateTime,
                <T::Repository as Repository>::DemandId,
            >,
        >,
    ),
    (StatusCode, String),
> {
    let as_of = app.now();
    let db = app.database();
    let demand_id = app.generate_demand_id(&body.app_data);
    let bidder_id = app
        .can_create_bid(&auth)
        .await
        .ok_or((StatusCode::UNAUTHORIZED, "not authorized".to_string()))?;

    db.create_demand(
        demand_id.clone(),
        bidder_id,
        body.app_data,
        body.curve_data,
        as_of.clone(),
    )
    .await
    .map(|_| {
        (
            StatusCode::CREATED,
            Json(CreateDemandResponseBody { as_of, demand_id }),
        )
    })
    .map_err(|err| {
        event!(Level::ERROR, err = err.to_string());
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to create demand".to_string(),
        )
    })
}

/// Retrieve a demand's current state.
///
/// Returns the demand data including its curve, associated portfolios,
/// and application-specific data.
///
/// # Authorization
///
/// Requires read permission for the demand's bidder (`can_read_bid`).
///
/// # Returns
///
/// - `200 OK`: Demand data
/// - `401 Unauthorized`: Missing read permissions
/// - `404 Not Found`: Demand does not exist
/// - `500 Internal Server Error`: Database query failed
async fn get_demand<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { demand_id }): Path<Id<<T::Repository as Repository>::DemandId>>,
) -> Result<Json<DemandRecord<T::Repository, T::DemandData>>, (StatusCode, String)> {
    let as_of = app.now();
    let db = app.database();
    let demand = db
        .get_demand(demand_id.clone(), as_of)
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get demand {}", demand_id),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("unknown demand {}", demand_id),
        ))?;

    if app.can_read_bid(&auth, demand.bidder_id.clone()).await {
        Ok(Json(demand))
    } else {
        Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()))
    }
}

/// Update a demand's curve data.
///
/// Replaces the existing curve with the provided curve. This creates a new
/// history entry while preserving previous curve data.
///
/// # Authorization
///
/// Requires update permission for the demand's bidder (`can_update_bid`).
///
/// # Returns
///
/// - `200 OK`: Demand updated successfully
/// - `401 Unauthorized`: Missing update permissions
/// - `404 Not Found`: Demand does not exist
/// - `500 Internal Server Error`: Database operation failed
async fn update_demand<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { demand_id }): Path<Id<<T::Repository as Repository>::DemandId>>,
    Json(body): Json<DemandCurve>,
) -> Result<Json<DemandRecord<T::Repository, T::DemandData>>, (StatusCode, String)> {
    let as_of = app.now();
    let db = app.database();

    // Check if the user is authorized to update the demand
    let bidder_id = db
        .get_demand_bidder_id(demand_id.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get demand {}", demand_id),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("unknown demand {}", demand_id),
        ))?;

    if !app.can_update_bid(&auth, bidder_id).await {
        return Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()));
    }

    let updated = db
        .update_demand(demand_id.clone(), Some(body), as_of.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to update demand {}", demand_id),
            )
        })?
        .ok_or_else(|| {
            event!(
                Level::ERROR,
                err = "failed to update demand after successful read"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to update demand {}", demand_id),
            )
        })?;

    Ok(Json(updated))
}

/// Delete a demand by setting its curve data to None.
///
/// This doesn't remove the demand from the database but deactivates it
/// by removing its curve data. The demand's history is preserved.
///
/// # Authorization
///
/// Requires update permission for the demand's bidder (`can_update_bid`).
///
/// # Returns
///
/// - `200 OK`: Demand deleted successfully
/// - `401 Unauthorized`: Missing update permissions
/// - `404 Not Found`: Demand does not exist
/// - `500 Internal Server Error`: Database operation failed
async fn delete_demand<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { demand_id }): Path<Id<<T::Repository as Repository>::DemandId>>,
) -> Result<Json<DemandRecord<T::Repository, T::DemandData>>, (StatusCode, String)> {
    let as_of = app.now();
    let db = app.database();

    // Check if the user is authorized to update the demand
    let bidder_id = db
        .get_demand_bidder_id(demand_id.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get demand {}", demand_id),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("unknown demand {}", demand_id),
        ))?;

    if !app.can_update_bid(&auth, bidder_id).await {
        return Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()));
    }

    let deleted = db
        .update_demand(demand_id.clone(), None, as_of.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to delete demand {}", demand_id),
            )
        })?
        .ok_or_else(|| {
            event!(
                Level::ERROR,
                err = "failed to delete demand after successful read"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to delete demand {}", demand_id),
            )
        })?;

    Ok(Json(deleted))
}

/// Retrieve the historical changes to a demand's curve data.
///
/// Returns a paginated list of curve changes over time, including when
/// the demand was created, updated, or deleted (curve set to None).
///
/// # Authorization
///
/// Requires read permission for the demand's bidder (`can_read_bid`).
///
/// # Returns
///
/// - `200 OK`: Paginated history records
/// - `401 Unauthorized`: Missing read permissions
/// - `404 Not Found`: Demand does not exist
/// - `500 Internal Server Error`: Database query failed
async fn get_demand_curve_history<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Path(Id { demand_id }): Path<Id<<T::Repository as Repository>::DemandId>>,
    Extension(config): Extension<Arc<config::AxumConfig>>,
    Query(query): Query<DateTimeRangeQuery<<T::Repository as Repository>::DateTime>>,
) -> Result<
    Json<DateTimeRangeResponse<DemandCurve, <T::Repository as Repository>::DateTime>>,
    (StatusCode, String),
> {
    let db = app.database();

    // Check if the user is authorized to read the demand history
    let bidder_id = db
        .get_demand_bidder_id(demand_id.clone())
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get demand {}", demand_id),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("unknown demand {}", demand_id),
        ))?;

    if !app.can_read_bid(&auth, bidder_id).await {
        return Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()));
    }
    let history = db
        .get_demand_curve_history(demand_id.clone(), query, config.page_limit)
        .await
        .map_err(|err| {
            event!(Level::ERROR, err = err.to_string());
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to get demand history {}", demand_id),
            )
        })?;

    Ok(Json(history))
}

/// Request body for creating a new demand.
#[derive(serde::Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
struct CreateDemandRequestBody<D> {
    /// Application-specific data to associate with the demand
    app_data: D,
    /// Optional initial curve data
    curve_data: Option<DemandCurve>,
}

/// Response body for creating a new demand.
#[derive(serde::Serialize, schemars::JsonSchema)]
#[schemars(inline)]
struct CreateDemandResponseBody<T, U> {
    /// The effective timestamp of the demand
    as_of: T,
    /// The system-generated id of the demand curve
    demand_id: U,
}
