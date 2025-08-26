//! REST API endpoints for product operations.
//!
//! This module provides operations for managing the product hierarchy, including
//! creating root products and partitioning them into child products. Products
//! represent the tradeable assets in the flow trading system.

use crate::ApiApplication;
use aide::axum::{
    ApiRouter,
    routing::{get, get_with, post},
};

mod crud;
use crud::*;

mod outcomes;
use outcomes::*;

/// Path parameter for product-specific endpoints.
#[derive(serde::Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
struct Id<T> {
    /// The unique identifier of the product
    product_id: T,
}

/// Creates a router with product-related endpoints.
pub fn router<T: ApiApplication>() -> ApiRouter<T> {
    ApiRouter::new()
        // TODO: figure out approach to querying!
        .api_route_with("/", post(create_product::<T>), |route| {
            route
                .security_requirement("jwt")
                .tag("product")
                .tag("admin")
        })
        .api_route(
            "/{product_id}",
            get_with(read_product::<T>, |route| {
                route.security_requirement("jwt").tag("product")
            })
            .post_with(update_product::<T>, |route| {
                route
                    .security_requirement("jwt")
                    .tag("product")
                    .tag("admin")
            }),
        )
        .api_route_with(
            "/{product_id}/outcomes",
            get(get_product_outcomes::<T>),
            |route| {
                route
                    .security_requirement("jwt")
                    .tag("product")
                    .tag("outcome")
            },
        )
}
