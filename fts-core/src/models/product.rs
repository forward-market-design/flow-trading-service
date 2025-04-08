use crate::models::ProductId;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{IntoParams, ToSchema};

/// A product record combines a standard product ID with implementation-specific data.
///
/// Products are the fundamental tradable entities in the system. The core only defines
/// products by their ID, allowing implementations to attach domain-specific data.
///
/// Product records are immutable once defined.
#[derive(Serialize, Deserialize, ToSchema)]
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
#[derive(Serialize, ToSchema)]
pub struct ProductQueryResponse<T, U> {
    /// Collection of product query results
    pub results: Vec<T>,
    /// Optional pagination information for retrieving additional results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub more: Option<U>,
}

/// A description of a product in a forward market
#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ProductData {
    /// A tag describing the type of product (e.g. "FORWARD" or "OPTION")
    pub kind: String,
    /// The starting time of the delivery interval for the product
    #[serde(with = "time::serde::rfc3339")]
    pub from: OffsetDateTime,
    /// The stopping time of the delivery interval for the product
    #[serde(with = "time::serde::rfc3339")]
    pub thru: OffsetDateTime,
}

/// A query for searching for known products
#[derive(Debug, Serialize, Deserialize, IntoParams, ToSchema)]
pub struct ProductQuery {
    /// An optional filter to restrict the kind of product by
    #[serde(default)]
    #[param(inline)]
    pub kind: Option<String>,
    /// An optional filter to select products with delivery windows on or before this value
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub before: Option<OffsetDateTime>,
    /// An optional filter to select products with delivery windows on or after this value
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub after: Option<OffsetDateTime>,
}
