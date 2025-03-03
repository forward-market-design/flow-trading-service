use crate::AppState;
use axum::{Router, routing};
use fts_core::{models::PortfolioDisplay, ports::MarketRepository};
use serde::Deserialize;
use utoipa::IntoParams;

pub mod delete;
pub mod get;
pub mod history;
pub mod list;
pub mod outcomes;
pub mod post;
pub mod put;

/// Every CRUD action takes a set of optional query parameters.
/// For now, there is only one such parameter, which determines
/// whether or not to return the associated portfolio with the
/// response.
#[derive(Deserialize, IntoParams)]
pub struct AuthParams {
    #[serde(default)]
    pub portfolio: PortfolioDisplay,
}

pub fn router<T: MarketRepository>() -> Router<AppState<T>> {
    Router::new()
        // TODO: add a GET handler for a new auth query endpoint
        .route("/", routing::get(list::list_auths).post(post::post_auth))
        .route(
            "/{auth_id}",
            routing::get(get::get_auth)
                .put(put::put_auth)
                .delete(delete::delete_auth),
        )
        .route("/{auth_id}/history", routing::get(history::get_history))
        .route("/{auth_id}/outcomes", routing::get(outcomes::get_outcomes))
}
