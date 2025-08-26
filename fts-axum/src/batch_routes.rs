//! REST API endpoints for batch auction operations.
//!
//! This module provides endpoints for executing batch auctions. Batch auctions
//! are the core mechanism through which flow trading clears the market by
//! solving for optimal allocations and prices at regular intervals.

use aide::axum::{ApiRouter, routing::post};
use axum::{extract::State, http::StatusCode};
use axum_extra::TypedHeader;
use fts_core::ports::BatchRepository as _;
use headers::{Authorization, authorization::Bearer};
use tracing::{Level, event};

use crate::ApiApplication;

/// Creates a router with batch-related endpoints.
pub fn router<T: ApiApplication>() -> ApiRouter<T> {
    ApiRouter::new().api_route_with("/", post(batch_solve::<T>), |route| {
        route.security_requirement("jwt").tag("admin")
    })
}

/// Execute a batch auction at the current timestamp.
///
/// This endpoint triggers the solver to process all active demands and portfolios,
/// computing optimal allocations and clearing prices. The results are persisted
/// in the database for later retrieval via the outcome endpoints.
///
/// # Authorization
///
/// Requires `can_run_batch` permission.
///
/// # Returns
///
/// - `200 OK`: Batch executed successfully, returns the timestamp
/// - `401 Unauthorized`: Missing or insufficient permissions
/// - `500 Internal Server Error`: Solver or database operation failed
async fn batch_solve<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    let as_of = app.now();
    if app.can_run_batch(&auth).await {
        let db = app.database();

        let _expires = db
            .run_batch(as_of.clone(), app.solver(), Default::default())
            .await
            .map_err(|err| {
                event!(Level::ERROR, err = err.to_string());
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to launch solver"),
                )
            })?
            .map_err(|err| {
                event!(Level::ERROR, err = err.to_string());
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to solve batch"),
                )
            })?;

        // assert _expires.is_none()?

        Ok((StatusCode::OK, format!("{}", as_of)))
    } else {
        Err((StatusCode::UNAUTHORIZED, "not authorized".to_string()))
    }
}
