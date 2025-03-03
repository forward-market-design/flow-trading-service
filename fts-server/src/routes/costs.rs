use crate::AppState;
use axum::{Router, routing};
use fts_core::{models::GroupDisplay, ports::MarketRepository};
use serde::Deserialize;
use utoipa::IntoParams;

pub mod delete;
pub mod get;
pub mod history;
pub mod list;
pub mod post;
pub mod put;

pub fn router<T: MarketRepository>() -> Router<AppState<T>> {
    Router::new()
        // TODO: add a GET handler for a new cost query endpoint
        .route("/", routing::get(list::list_costs).post(post::post_cost))
        .route(
            "/{cost_id}",
            routing::get(get::get_cost)
                .put(put::put_cost)
                .delete(delete::delete_cost),
        )
        .route("/{cost_id}/history", routing::get(history::get_history))
}

#[derive(Deserialize, IntoParams)]
pub struct CostParams {
    #[serde(default)]
    pub group: GroupDisplay,
}
