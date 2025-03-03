use crate::AppState;
use axum::{extract::FromRequestParts, http::request::Parts, response::sse::Event};
use fts_core::{models::ProductId, ports::MarketRepository};
use fxhash::FxBuildHasher;
use std::{convert::Infallible, sync::Arc};
use tokio::sync::watch;

/// Extract a receiver from the app state
pub struct ActivityReceiver(pub watch::Receiver<Result<Event, Infallible>>);

impl<T: MarketRepository> FromRequestParts<AppState<T>> for ActivityReceiver
where
    AppState<T>: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        _: &mut Parts,
        state: &AppState<T>,
    ) -> Result<Self, Self::Rejection> {
        Ok(ActivityReceiver(state.activity_receiver.clone()))
    }
}

type Sender<T> = Arc<dashmap::DashMap<T, watch::Sender<Result<Event, Infallible>>, FxBuildHasher>>;

pub struct ProductSender(pub Sender<ProductId>);

impl<T: MarketRepository> FromRequestParts<AppState<T>> for ProductSender
where
    AppState<T>: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        _: &mut Parts,
        state: &AppState<T>,
    ) -> Result<Self, Self::Rejection> {
        Ok(ProductSender(state.product_sender.clone()))
    }
}
