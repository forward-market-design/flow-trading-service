use axum::Router;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{IntoParams, Modify, OpenApi, ToSchema};
use utoipa_rapidoc::RapiDoc;

use fts_core::models::{AuthHistoryRecord, CostHistoryRecord, DateTimeRangeQuery, ProductId};

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
/// The product definition is generic, so this serves just as an example.
pub struct ProductData {
    pub example_attribute: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
/// As the product definition is generic, the query to retrieve these products is also generic.
pub struct ProductQuery {
    pub example_query: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExampleAuthHistoryResponse {
    results: Vec<AuthHistoryRecord>,
    more: Option<DateTimeRangeQuery>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExampleCostHistoryResponse {
    results: Vec<CostHistoryRecord>,
    more: Option<DateTimeRangeQuery>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExampleProductRecord {
    pub id: ProductId,
    pub data: ProductData,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExampleProductQueryResponse {
    results: Vec<ExampleProductRecord>,
    more: Option<ProductQuery>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExampleOutcome {
    pub price: f64,
    pub trade: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Additional data; Defined by domain, not necessarily a string.")]
    pub data: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExampleAuctionOutcome {
    pub from: OffsetDateTime,
    pub thru: OffsetDateTime,
    pub outcome: ExampleOutcome,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExampleAuctionOutcomeResponse {
    results: Vec<ExampleAuctionOutcome>,
    more: Option<DateTimeRangeQuery>,
}

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum Error {
    #[error("Example error: {0}")]
    ExampleError(String),
}

#[derive(OpenApi)]
#[openapi(
    paths(
        super::routes::admin::define_products,
        super::routes::admin::solve::solve_auctions,
        super::routes::products::list_products,
        super::routes::products::get_product,
        super::routes::products::product_outcomes,
        super::routes::submission::get_submission,
        super::routes::submission::put_submission,
        super::routes::submission::delete_submission,
        super::routes::auths::post::post_auth,
        super::routes::auths::get::get_auth,
        super::routes::auths::put::put_auth,
        super::routes::auths::delete::delete_auth,
        super::routes::auths::list::list_auths,
        super::routes::auths::outcomes::get_outcomes,
        super::routes::auths::history::get_history,
        super::routes::costs::post::post_cost,
        super::routes::costs::get::get_cost,
        super::routes::costs::put::put_cost,
        super::routes::costs::delete::delete_cost,
        super::routes::costs::history::get_history,
    ),
    info(
        description = include_str!("../API_REFERENCE.md")
    ),
    external_docs(
        url = "https://forwardmarketdesign.com", description = "📖 Flow Trading Introduction"
    ),
    modifiers(&SecurityAddon),
    security(
        ("jwt" = []),
    )
)]
pub struct MarketplaceApi;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap(); // we can unwrap safely since there already is components registered.
        components.add_security_scheme(
            "jwt",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        )
    }
}
pub fn openapi_router() -> Router {
    RapiDoc::with_url(
        "/rapidoc",
        "/api-docs/openapi.json",
        MarketplaceApi::openapi(),
    )
    // rapidoc can be customized according to https://rapidocweb.com/api.html
    .custom_html(
        r#"
<!doctype html> <!-- Important: must specify -->
<html>
  <head>
    <meta charset="utf-8"> <!-- Important: rapi-doc uses utf8 characters -->
    <script type="module" src="https://unpkg.com/rapidoc/dist/rapidoc-min.js"></script>
  </head>
  <body>
    <rapi-doc spec-url = $specUrl
        show-method-in-nav-bar = "as-colored-text"
        use-path-in-nav-bar = "true"
    ></rapi-doc>
  </body>
</html>
"#,
    )
    .into()
}
