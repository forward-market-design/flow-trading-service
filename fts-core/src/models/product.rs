use crate::models::ProductId;
use serde::{Deserialize, Serialize};

/// A product record combines a standard product ID with implementation-specific data.
///
/// Products are the fundamental tradable entities in the system. The core only defines
/// products by their ID, allowing implementations to attach domain-specific data.
///
/// Product records are immutable once defined.
#[derive(Serialize, Deserialize)]
pub struct ProductRecord<T> {
    /// Unique identifier for the product
    pub id: ProductId,
    /// Additional product-specific data defined by the implementation
    #[serde(flatten)]
    pub data: T,
}

/// Standard response format for product queries.
///
/// This structure provides a consistent format for returning product query results,
/// including optional pagination metadata.
#[derive(Serialize)]
pub struct ProductQueryResponse<T, U> {
    /// Collection of product query results
    pub results: Vec<T>,
    /// Optional pagination information for retrieving additional results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub more: Option<U>,
}
