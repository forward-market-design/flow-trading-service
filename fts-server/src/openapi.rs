use axum::Router;
use thiserror::Error;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_rapidoc::RapiDoc;

use fts_core::models::GroupDisplay;

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
    components(schemas(
        GroupDisplay, // This is not being pulled in automatically, adding it manually
    )),
    external_docs(
        url = "https://flowtrading.forwardmarketdesign.com", description = "📖 Flow Trading Introduction"
    ),
    modifiers(&SecurityAddon),
    security(
        ("jwt" = []),
    )
)]
/// The OpenAPI spec for the Flow Trading System
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
    <script src="https://cdnjs.cloudflare.com/ajax/libs/rapidoc/9.3.8/rapidoc-min.js" integrity="sha512-0ES6eX4K9J1PrIEjIizv79dTlN5HwI2GW9Ku6ymb8dijMHF5CIplkS8N0iFJ/wl3GybCSqBJu8HDhiFkZRAf0g==" crossorigin="anonymous" referrerpolicy="no-referrer"></script>
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
