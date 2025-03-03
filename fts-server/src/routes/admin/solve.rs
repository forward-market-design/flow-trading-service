use crate::{AppState, Now};
use axum::{Json, extract::State, http::StatusCode};
use fts_core::{
    models::AuctionSolveRequest,
    ports::{AuctionRepository, MarketRepository},
};
use tracing::{Level, event};

/// Solve an auction
///
/// This function handles the endpoint for solving auctions. It processes the auction solve request,
/// aggregates authorizations and costs by bidder, and prepares submissions for the solver. The submissions
/// are then sent to the solve queue for processing.
#[utoipa::path(
    post,
    path = "/admin/auctions/solve",
    request_body = AuctionSolveRequest,
    responses(
        (status = ACCEPTED, description = "Auction(s) initiated"),
        (status = BAD_REQUEST, description = "Invalid datetime range"),
        (status = INTERNAL_SERVER_ERROR)
    ),
    tags = ["admin"]
)]
pub async fn solve_auctions<T: MarketRepository>(
    State(state): State<AppState<T>>,
    Now(now): Now,
    Json(auction): Json<AuctionSolveRequest>,
) -> StatusCode {
    // Get the inputs for the auction
    let inputs =
        AuctionRepository::prepare(&state.market, auction.from, auction.thru, auction.by, now)
            .await;

    match inputs {
        // Did we successfully create an auction? If so, queue it up for solution
        Ok(Some(auctions)) => {
            for auction in auctions {
                match state.solve_queue.send(auction).await {
                    Ok(_) => {}
                    Err(e) => {
                        event!(Level::ERROR, error = ?e);
                        return StatusCode::INTERNAL_SERVER_ERROR;
                    }
                }
            }
            StatusCode::ACCEPTED
        }
        // No database errors, but did we fail to schedule an auction?
        Ok(None) => StatusCode::BAD_REQUEST,
        // Database errors!
        Err(e) => {
            event!(Level::ERROR, error = ?e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
