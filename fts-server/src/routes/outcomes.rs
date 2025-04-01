use crate::{
    AppState,
    utils::{ActivityReceiver, Bidder, ProductSender},
};
use axum::{
    Router,
    extract::{Path, State},
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
use tokio::sync::watch;
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
    ProductSender(sender): ProductSender,
) -> Sse<WatchStream<Result<Event, Infallible>>> {
    let rcv = match sender.entry(product_id) {
        dashmap::Entry::Occupied(entry) => entry.get().subscribe(),
        dashmap::Entry::Vacant(entry) => {
            let (snd, rcv) = watch::channel(Ok(Event::default().comment("")));
            entry.insert(snd);
            rcv
        }
    };

    Sse::new(WatchStream::new(rcv)).keep_alive(KeepAlive::default())
}

async fn bidder_stream<T: MarketRepository>(
    Bidder(bidder_id): Bidder,
    State(state): State<AppState<T>>,
    Path(bidder_id2): Path<BidderId>,
) -> Result<Sse<WatchStream<Result<Event, Infallible>>>, StatusCode> {
    if bidder_id != bidder_id2 {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let rcv = match state.bidder_sender.entry(bidder_id) {
        dashmap::Entry::Occupied(entry) => entry.get().subscribe(),
        dashmap::Entry::Vacant(entry) => {
            let (snd, rcv) = watch::channel(Ok(Event::default().comment("")));
            entry.insert(snd);
            rcv
        }
    };

    Ok(Sse::new(WatchStream::new(rcv)).keep_alive(KeepAlive::default()))
}
