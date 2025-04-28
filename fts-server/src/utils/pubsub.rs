use crate::AppState;
use axum::{extract::FromRequestParts, http::request::Parts, response::sse::Event};
use fts_core::{
    models::{BidderId, ProductId},
    ports::MarketRepository,
};
use rustc_hash::FxBuildHasher;
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

impl ProductSender {
    pub fn get_receiver(
        &self,
        product_id: ProductId,
    ) -> watch::Receiver<Result<Event, Infallible>> {
        match self.0.entry(product_id) {
            dashmap::Entry::Occupied(entry) => entry.get().subscribe(),
            dashmap::Entry::Vacant(entry) => {
                let (snd, rcv) = watch::channel(Ok(Event::default().comment("")));
                entry.insert(snd);
                rcv
            }
        }
    }
}

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

pub struct BidderSender(pub Sender<BidderId>);

impl BidderSender {
    pub fn get_receiver(&self, bidder_id: BidderId) -> watch::Receiver<Result<Event, Infallible>> {
        match self.0.entry(bidder_id) {
            dashmap::Entry::Occupied(entry) => entry.get().subscribe(),
            dashmap::Entry::Vacant(entry) => {
                let (snd, rcv) = watch::channel(Ok(Event::default().comment("")));
                entry.insert(snd);
                rcv
            }
        }
    }
}

impl<T: MarketRepository> FromRequestParts<AppState<T>> for BidderSender
where
    AppState<T>: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        _: &mut Parts,
        state: &AppState<T>,
    ) -> Result<Self, Self::Rejection> {
        Ok(BidderSender(state.bidder_sender.clone()))
    }
}
