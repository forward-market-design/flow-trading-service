use utoipa::OpenApi;

fn main() {
    // Get the OpenAPI spec from the server crate
    let openapi = fts_server::MarketplaceApi::openapi();

    // Convert to YAML
    let yaml = openapi
        .to_yaml()
        .expect("Failed to convert OpenAPI spec to YAML");

    println!("{}", yaml);
}
