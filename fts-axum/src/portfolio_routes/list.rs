use axum::{Json, extract::State, http::StatusCode};
use axum_extra::TypedHeader;
use fts_core::{models::PortfolioRecord, ports::PortfolioRepository as _};
use headers::{Authorization, authorization::Bearer};
use tracing::{Level, event};

use crate::ApiApplication;

pub(crate) async fn list_portfolios<T: ApiApplication>(
    State(app): State<T>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Vec<PortfolioRecord<T::Repository, T::PortfolioData>>>, StatusCode> {
    let db = app.database();
    let bidder_ids = app.can_query_bid(&auth).await;

    if bidder_ids.is_empty() {
        Err(StatusCode::UNAUTHORIZED)
    } else {
        Ok(Json(db.query_portfolio(&bidder_ids).await.map_err(
            |err| {
                event!(Level::ERROR, err = err.to_string());
                StatusCode::INTERNAL_SERVER_ERROR
            },
        )?))
    }
}
