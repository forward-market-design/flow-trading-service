use std::fs;
use std::path::Path;
use utoipa::OpenApi;

fn main() {
    // Get the OpenAPI spec from the server crate
    let openapi = fts_server::MarketplaceApi::openapi();

    // Convert to YAML
    let yaml = openapi
        .to_yaml()
        .expect("Failed to convert OpenAPI spec to YAML");

    // Write to file
    // Parse command line arguments for a custom destination path
    let dest_path = std::env::args()
        .nth(1)
        .map(|s| Path::new(&s).to_path_buf())
        .unwrap_or_else(|| {
            // Get the manifest directory first
            let manifest_dir =
                std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

            // Get the parent of the manifest directory
            let parent_dir = Path::new(&manifest_dir)
                .parent()
                .expect("Failed to get parent of manifest directory");

            // Create target directory in the parent directory
            let target_dir = parent_dir.join("target");
            fs::create_dir_all(&target_dir).expect("Failed to create target directory");
            target_dir.join("openapi.yaml")
        });
    fs::write(&dest_path, yaml).expect("Failed to write OpenAPI YAML file");

    println!("Generated OpenAPI spec at {}", dest_path.display());
}
