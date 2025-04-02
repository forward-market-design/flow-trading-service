use crate::{
    AppState,
    utils::{ActivityReceiver, Bidder, BidderSender, ProductSender},
};
use axum::{
    Router,
    extract::Path,
    http::StatusCode,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
    routing,
};
use fts_core::{
    models::{BidderId, ProductId},
    ports::MarketRepository,
};
use std::convert::Infallible;
use tokio_stream::wrappers::WatchStream;

pub fn router<T: MarketRepository>() -> Router<AppState<T>> {
    Router::new()
        // Query the product directory
        .route("/", routing::get(activity_stream))
        // Get all data for a certain product
        .route("/products/{product_id}", routing::get(product_stream))
        // View the results associated to a product
        .route("/bidders/{bidder_id}", routing::get(bidder_stream))
}

async fn activity_stream(
    ActivityReceiver(receiver): ActivityReceiver,
) -> Sse<WatchStream<Result<Event, Infallible>>> {
    Sse::new(WatchStream::new(receiver)).keep_alive(KeepAlive::default())
}

async fn product_stream(
    Path(product_id): Path<ProductId>,
    stream: ProductSender,
) -> Sse<WatchStream<Result<Event, Infallible>>> {
    Sse::new(WatchStream::new(stream.get_receiver(product_id))).keep_alive(KeepAlive::default())
}

async fn bidder_stream(
    Bidder(bidder_id): Bidder,
    Path(bidder_id2): Path<BidderId>,
    stream: BidderSender,
) -> Result<Sse<WatchStream<Result<Event, Infallible>>>, StatusCode> {
    if bidder_id == bidder_id2 {
        Ok(Sse::new(WatchStream::new(stream.get_receiver(bidder_id)))
            .keep_alive(KeepAlive::default()))
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
