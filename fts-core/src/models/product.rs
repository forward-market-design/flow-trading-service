use crate::models::ProductId;
use serde::{Deserialize, Serialize};

/// As far as the core is concerned, a product is its id. However, an implementation
/// likely has additional data to concern itself with.
#[derive(Serialize, Deserialize)]
pub struct ProductRecord<T> {
    pub id: ProductId,
    #[serde(flatten)]
    pub data: T,
}

/// Depending on the product structure, an implementation may have a rich and complex way
/// to respond to product queries. This enforces a baseline structure on these responses.
#[derive(Serialize)]
pub struct ProductQueryResponse<T, U> {
    pub results: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub more: Option<U>,
}
