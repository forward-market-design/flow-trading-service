use crate::{
    AppState, Now,
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
    models::{AuthId, ProductId},
    ports::{AuthFailure, AuthRepository, MarketRepository},
};
use std::convert::Infallible;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use tracing::{Level, event};

pub fn router<T: MarketRepository>() -> Router<AppState<T>> {
    Router::new()
        // Query the product directory
        .route("/", routing::get(activity_stream))
        // Get all data for a certain product
        .route("/products/{product_id}", routing::get(product_stream))
        // View the results associated to a product
        .route("/auths/{auth_id}", routing::get(auth_stream))
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

async fn auth_stream<T: MarketRepository>(
    Bidder(bidder_id): Bidder,
    State(state): State<AppState<T>>,
    Path(auth_id): Path<AuthId>,
    Now(now): Now,
) -> Result<Sse<WatchStream<Result<Event, Infallible>>>, StatusCode> {
    // First, check to see if we can read the auth itself.
    // We assume that being able to view an auth = able to see results
    let _ = AuthRepository::read(
        &state.market,
        bidder_id,
        auth_id,
        now,
        T::PortfolioOptions::default(),
    )
    .await
    .map_err(|err| {
        event!(Level::ERROR, error = ?err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .map_err(|err| match err {
        // For some reason the implementation does not consider the bidder to have adequate permissions
        AuthFailure::AccessDenied => StatusCode::FORBIDDEN,
        // The auth does not exist
        AuthFailure::DoesNotExist => StatusCode::NOT_FOUND,
        // This value should probably never be returned.
        AuthFailure::IdConflict => {
            event!(Level::ERROR, error = "auth read returned IdConflict");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let rcv = match state.auth_sender.entry(auth_id) {
        dashmap::Entry::Occupied(entry) => entry.get().subscribe(),
        dashmap::Entry::Vacant(entry) => {
            let (snd, rcv) = watch::channel(Ok(Event::default().comment("")));
            entry.insert(snd);
            rcv
        }
    };

    Ok(Sse::new(WatchStream::new(rcv)).keep_alive(KeepAlive::default()))
}
