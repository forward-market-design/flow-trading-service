use crate::models::ProductId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ProductRecord<T> {
    pub id: ProductId,
    #[serde(flatten)]
    pub data: T,
}

#[derive(Serialize)]
pub struct ProductQueryResponse<T, U> {
    pub results: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub more: Option<U>,
}
