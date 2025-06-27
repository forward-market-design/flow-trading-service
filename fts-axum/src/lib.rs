#![warn(missing_docs)]
// Note: this overwrites the link in the README to point to the rust docs of the fts-sqlite crate.
//! [fts_core]: https://docs.rs/fts_core/latest/fts_core/index.html
//! [fts_axum]: https://docs.rs/fts_axum/latest/fts_axum/index.html
//! [fts_solver]: https://docs.rs/fts_solver/latest/fts_solver/index.html
//! [fts_sqlite]: https://docs.rs/fts_sqlite/latest/fts_sqlite/index.html
#![doc = include_str!("../README.md")]

mod batch_routes;
mod demand_routes;
mod portfolio_routes;
mod product_routes;

use aide::{
    axum::{ApiRouter, routing::get},
    openapi::OpenApi,
};
use axum::{Extension, Json};
use fts_core::ports::{Application, Repository, Solver};
use headers::{Authorization, authorization::Bearer};
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};
use std::{fmt::Display, sync::Arc};

mod openapi;
use openapi::{api_docs, docs_routes};

pub mod config;
use config::AxumConfig;

/// Response for the health check endpoint
#[derive(Serialize, JsonSchema)]
#[schemars(inline)]
struct HealthResponse {
    status: String,
}

/// Simple health check endpoint
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

/// Construct a full API router with the given state and config
pub fn router<T: ApiApplication>(state: T, config: AxumConfig) -> axum::Router {
    let mut api = OpenApi::default();
    ApiRouter::new()
        .api_route("/health", get(health_check))
        .nest("/product", product_routes::router())
        .nest("/demand", demand_routes::router())
        .nest("/portfolio", portfolio_routes::router())
        .nest("/batch", batch_routes::router())
        .nest_api_service("/docs", docs_routes())
        .finish_api_with(&mut api, api_docs)
        .layer(Extension(Arc::new(api))) // Arc is very important here or you will face massive memory and performance issues
        .layer(Extension(Arc::new(config)))
        .with_state(state)
}

/// Starts the HTTP server with the provided configuration
pub async fn start_server<T: ApiApplication>(
    config: AxumConfig,
    app: T,
) -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind(config.bind_address)
        .await
        .expect("Unable to bind to address");

    tracing::info!(
        "Listening for requests on {}",
        listener.local_addr().unwrap()
    );

    // Here, we could apply additional config like timeouts, CORS, etc.
    let service = router(app, config);
    axum::serve(listener, service).await
}

/// Axum imposes all sorts of constraints on what can pass for state. This
/// trait, coupled with a blanket implementation, specifies it all upfront and
/// in one place. If a function takes a generic `T: ApiApplication`, then
/// everything one might reasonably want to do should work.
pub trait ApiApplication:
    Clone
    + Send
    + Sync
    + 'static
    + Application<
        Context = Authorization<Bearer>,
        DemandData: Send + Sync + Serialize + DeserializeOwned + JsonSchema + 'static,
        PortfolioData: Send + Sync + Serialize + DeserializeOwned + JsonSchema + 'static,
        ProductData: Send + Sync + Serialize + DeserializeOwned + JsonSchema + 'static,
        Repository: Clone
                        + Send
                        + Sync
                        + 'static
                        + Repository<
            DateTime: Clone + Display + Serialize + DeserializeOwned + JsonSchema + Send + Sync,
            BidderId: Clone + Display + Serialize + DeserializeOwned + JsonSchema + Send + Sync,
            DemandId: Clone + Display + Serialize + DeserializeOwned + JsonSchema + Send + Sync,
            PortfolioId: Clone + Display + Serialize + DeserializeOwned + JsonSchema + Send + Sync,
            ProductId: Clone + Display + Serialize + DeserializeOwned + JsonSchema + Send + Sync,
        >,
        Solver: Solver<
            <Self::Repository as Repository>::DemandId,
            <Self::Repository as Repository>::PortfolioId,
            <Self::Repository as Repository>::ProductId,
            PortfolioOutcome: Send + Sync + Serialize + DeserializeOwned + JsonSchema + 'static,
            ProductOutcome: Send + Sync + Serialize + DeserializeOwned + JsonSchema + 'static,
        >,
    >
{
}

// this is the blanket implementation
impl<T: Clone + Send + Sync + 'static> ApiApplication for T where
    T: Application<
            Context = Authorization<Bearer>,
            DemandData: Send + Sync + Serialize + DeserializeOwned + JsonSchema + 'static,
            PortfolioData: Send + Sync + Serialize + DeserializeOwned + JsonSchema + 'static,
            ProductData: Send + Sync + Serialize + DeserializeOwned + JsonSchema + 'static,
            Repository: Clone
                            + Send
                            + Sync
                            + 'static
                            + Repository<
                DateTime: Clone + Display + Serialize + DeserializeOwned + JsonSchema + Send + Sync,
                BidderId: Clone + Display + Serialize + DeserializeOwned + JsonSchema + Send + Sync,
                DemandId: Clone + Display + Serialize + DeserializeOwned + JsonSchema + Send + Sync,
                PortfolioId: Clone
                                 + Display
                                 + Serialize
                                 + DeserializeOwned
                                 + JsonSchema
                                 + Send
                                 + Sync,
                ProductId: Clone
                               + Display
                               + Serialize
                               + DeserializeOwned
                               + JsonSchema
                               + Send
                               + Sync,
            >,
            Solver: Solver<
                <T::Repository as Repository>::DemandId,
                <T::Repository as Repository>::PortfolioId,
                <T::Repository as Repository>::ProductId,
                PortfolioOutcome: Send + Sync + Serialize + DeserializeOwned + JsonSchema + 'static,
                ProductOutcome: Send + Sync + Serialize + DeserializeOwned + JsonSchema + 'static,
            >,
        >
{
}
