//! OpenAPI documentation generation and serving.
//!
//! This module provides endpoints for serving the auto-generated OpenAPI
//! specification and an interactive API documentation interface using RapiDoc.

use std::sync::Arc;

use aide::{
    axum::{ApiRouter, IntoApiResponse, routing::get},
    openapi::{OpenApi, Tag},
    transform::TransformOpenApi,
};
use axum::{
    Extension, Json,
    response::{Html, IntoResponse},
};
use serde::Serialize;

/// Serve the RapiDoc interactive API documentation interface.
///
/// Returns an HTML page that renders the OpenAPI specification using RapiDoc,
/// providing an interactive way to explore and test the API endpoints.
async fn serve_rapidoc() -> impl IntoApiResponse {
    let html = r#"<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <script src="https://cdnjs.cloudflare.com/ajax/libs/rapidoc/9.3.8/rapidoc-min.js" integrity="sha512-0ES6eX4K9J1PrIEjIizv79dTlN5HwI2GW9Ku6ymb8dijMHF5CIplkS8N0iFJ/wl3GybCSqBJu8HDhiFkZRAf0g==" crossorigin="anonymous" referrerpolicy="no-referrer"></script>
  </head>
  <body>
    <rapi-doc spec-url="/docs/api.json"
        show-method-in-nav-bar="as-colored-text"
        use-path-in-nav-bar="true"
    ></rapi-doc>
  </body>
</html>"#;
    Html(html).into_response()
}

/// Creates a router for documentation endpoints.
pub(crate) fn docs_routes() -> ApiRouter {
    let router: ApiRouter = ApiRouter::new()
        .route("/", get(serve_rapidoc))
        .route("/api.json", get(serve_docs));

    router
}

/// Wrapper to make Arc<OpenApi> serializable
#[derive(Clone)]
struct SerializableOpenApi(Arc<OpenApi>);

impl Serialize for SerializableOpenApi {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.as_ref().serialize(serializer)
    }
}

/// Serve the raw OpenAPI specification.
///
/// Returns the complete OpenAPI specification as JSON, which can be used
/// by API clients for code generation or other tooling.
async fn serve_docs(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(SerializableOpenApi(api)).into_response()
}

/// Configure the OpenAPI documentation metadata.
pub(crate) fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
    api.title("Flow Trading Service API")
        .summary("REST API for flow trading operations")
        .description("This API provides endpoints for managing demands, portfolios, products, and executing batch auctions in a flow trading system.")
        .tag(Tag {
            name: "demands".into(),
            description: Some("Operations on demand curves".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "portfolios".into(),
            description: Some("Operations on portfolios".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "products".into(),
            description: Some("Operations on tradeable products".into()),
            ..Default::default()
        })
        .tag(Tag {
            name: "batch".into(),
            description: Some("Batch auction execution".into()),
            ..Default::default()
        })
}
