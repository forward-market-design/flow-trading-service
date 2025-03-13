// Note: this overwrites the link in the README to point to the rust docs of the fts-demo crate.
//! [fts_core]: https://docs.rs/fts_core/latest/fts_core/index.html
//! [fts_server]: https://docs.rs/fts_server/latest/fts_server/index.html
//! [fts_solver]: https://docs.rs/fts_solver/latest/fts_solver/index.html
//! [fts_demo]: https://docs.rs/fts_demo/latest/fts_demo/index.html
#![doc = include_str!("../../docs/workspace.md")]
#![doc = include_str!("../README.md")]
use fts_core::{
    models::{AuthId, Outcome, ProductId, RawAuctionInput},
    ports::{AuctionRepository, MarketRepository},
};

use axum::Router;
use axum::http::header;
use axum::response::sse::Event;
use fts_solver::Solver;
use fxhash::FxBuildHasher;
use openapi::openapi_router;
use serde::Serialize;
use std::sync::Arc;
use std::{convert::Infallible, net::SocketAddr};
use time::OffsetDateTime;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::try_join;
use tower_http::cors;

mod openapi;
mod routes;
mod utils;

pub use utils::CustomJWTClaims;
use utils::JWTVerifier;
pub use utils::Now;
pub use utils::generate_jwt;

type SenderMap<T> =
    Arc<dashmap::DashMap<T, watch::Sender<Result<Event, Infallible>>, FxBuildHasher>>;

#[derive(Clone)]
pub struct AppState<T: MarketRepository> {
    jwt: JWTVerifier,
    market: T,
    solve_queue: mpsc::Sender<RawAuctionInput<T::AuctionId>>,
    activity_receiver: watch::Receiver<Result<Event, Infallible>>,
    product_sender: SenderMap<ProductId>,
    auth_sender: SenderMap<AuthId>,
}

#[derive(Serialize)]
pub struct Update {
    #[serde(with = "time::serde::rfc3339")]
    pub from: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub thru: OffsetDateTime,
    #[serde(flatten)]
    pub outcome: Outcome<()>,
}

pub fn state<T: MarketRepository>(
    api_secret: &str,
    market: T,
) -> (AppState<T>, JoinHandle<Result<(), T::Error>>) {
    // We create a FIFO queue for solving auctions
    let (solve_sender, mut solve_receiver) = mpsc::channel::<RawAuctionInput<T::AuctionId>>(24);

    // These channels are for reporting activity to SSE subscribers on /activity
    let (activity_sender, activity_receiver) = watch::channel(Ok(Event::default().comment("")));

    let product_sender: SenderMap<ProductId> = Default::default();
    let auth_sender: SenderMap<AuthId> = Default::default();

    let solver = {
        let market = market.clone();
        let activity_sender = activity_sender.clone();
        let product_sender = product_sender.clone();
        let auth_sender = auth_sender.clone();
        tokio::spawn(async move {
            let solver = T::solver();
            while let Some(auction) = solve_receiver.recv().await {
                let id = auction.id.clone();

                // Convert the auction into a format the solver understands
                let submissions: Vec<fts_solver::Submission<_, _>> = auction.into();

                // TODO: this is where warm-starting would be used
                let fts_solver::AuctionOutcome { auths, products } = solver.solve(&submissions);

                // TODO: update the API to scope the auth_id the bidder_id
                let auth_outcomes = auths
                    .iter()
                    .map(|(auth_id, outcome)| {
                        (
                            auth_id.clone(),
                            Outcome {
                                price: outcome.price,
                                trade: outcome.trade,
                                data: None,
                            },
                        )
                    })
                    .collect::<Vec<_>>();

                let product_outcomes = products
                    .iter()
                    .map(|(id, outcome)| {
                        (
                            id.clone(),
                            Outcome {
                                price: outcome.price,
                                trade: outcome.volume,
                                data: None,
                            },
                        )
                    })
                    .collect::<Vec<_>>();

                let now = OffsetDateTime::now_utc().into();

                // We are basically copy/pasting solver::*Outcome into crate::Outcome, which seems silly.
                // But, if the server wants to augment or transform the data somehow, this a reasonable
                // indirection to have.
                let metadata = AuctionRepository::report(
                    &market.clone(),
                    id,
                    auth_outcomes.into_iter(),
                    product_outcomes.into_iter(),
                    now,
                )
                .await?;

                // Now that we've stored the outcomes, we push updates to the SSE listeners
                if let Some(metadata) = metadata {
                    let _ = activity_sender.send_replace(Ok(Event::default()
                        .event("outcome")
                        .data(serde_json::to_string(&metadata).expect("infallible!"))));

                    // We also individually broadcast the results to any listeners.
                    // TODO: think about how to cleanup the dashmap over time
                    for (product_id, product_outcome) in products {
                        if let Some(channel) = product_sender.get(&product_id) {
                            let update = Update {
                                from: metadata.from,
                                thru: metadata.thru,
                                outcome: Outcome {
                                    price: product_outcome.price,
                                    trade: product_outcome.volume,
                                    data: None,
                                },
                            };
                            let _ = channel.send_replace(Ok(Event::default()
                                .event("outcome")
                                .data(serde_json::to_string(&update).expect("infallible!"))));
                        };
                    }

                    for (auth_id, auth_outcome) in auths {
                        if let Some(channel) = auth_sender.get(&auth_id) {
                            let update = Update {
                                from: metadata.from,
                                thru: metadata.thru,
                                outcome: Outcome {
                                    price: auth_outcome.price,
                                    trade: auth_outcome.trade,
                                    data: None,
                                },
                            };
                            let _ = channel.send_replace(Ok(Event::default()
                                .event("outcome")
                                .data(serde_json::to_string(&update).expect("infallible!"))));
                        }
                    }
                }
            }
            Result::<(), T::Error>::Ok(())
        })
    };

    let state = AppState {
        jwt: JWTVerifier::from(api_secret),
        market,
        solve_queue: solve_sender,
        activity_receiver,
        product_sender,
        auth_sender,
    };

    (state, solver)
}

pub fn router<T: MarketRepository>(state: AppState<T>) -> Router {
    // To allow for web app access, we use a permissive CORS policy. Notably,
    // this strips any implicit authorization, making this a pretty decent policy.
    let policy = cors::CorsLayer::new()
        .allow_origin(cors::Any)
        .allow_methods(cors::Any)
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    // Wire it together
    let app = Router::new()
        .nest("/v0/auths", routes::auths::router())
        .nest("/v0/costs", routes::costs::router())
        // Bidder-specific routes for their submission
        .nest("/v0/submissions", routes::submission::router(state.clone()))
        // View products and their results
        .nest("/v0/products", routes::products::router())
        // These are the SSE-routes for live-updates
        .nest("/v0/outcomes", routes::outcomes::router())
        .nest("/admin", routes::admin::router(state.clone()));

    app.layer(policy).with_state(state)
}

// The binary can simply provide the configuration to this function to launch
// the server
pub async fn start<T: MarketRepository>(api_port: u16, api_secret: String, market: T) {
    // Setup the HTTP server
    let listener = tokio::net::TcpListener::bind(SocketAddr::new([0, 0, 0, 0].into(), api_port))
        .await
        .expect("Unable to bind local port");
    tracing::info!(
        "Listening for requests on {}",
        listener.local_addr().unwrap()
    );

    let (app_state, solver) = state(&api_secret, market);

    // Setup the combined application state and serve it with our router
    let server = tokio::spawn(async move {
        axum::serve(listener, router(app_state).merge(openapi_router())).await
    });

    let _ = try_join!(solver, server).expect("shutdown");
}
