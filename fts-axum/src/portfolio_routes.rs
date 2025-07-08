//! REST API endpoints for portfolio operations.
//!
//! This module provides CRUD operations for portfolios, which aggregate demands
//! and associate them with tradeable products. Portfolios enable complex trading
//! strategies by allowing weighted combinations of demands and products.

use crate::ApiApplication;
use aide::{
    axum::{
        ApiRouter,
        routing::{get, get_with},
    },
    transform::TransformOperation,
};

mod crud;
use crud::*;

mod history;
use history::*;

mod list;
use list::*;

mod outcomes;
use outcomes::*;

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
            get_with(list_portfolios::<T>, list_portfolios_docs)
                .post_with(create_portfolio::<T>, create_portfolio_docs),
            |route| route.security_requirement("jwt").tag("portfolio"),
        )
        .api_route_with(
            "/{portfolio_id}",
            get(read_portfolio::<T>)
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

fn list_portfolios_docs(op: TransformOperation) -> TransformOperation<'_> {
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
