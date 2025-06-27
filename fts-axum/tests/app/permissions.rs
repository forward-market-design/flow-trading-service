use fts_sqlite::types::BidderId;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

// In order to test the correctness of our permission checks in our endpoints,
// we define a declarative permission scheme, which is encoded as plain text
// into the `Authorization: Bearer <...>` header. This allows us to easily
// construct "tokens" that exercise complex permission configurations.
#[derive(Serialize, Deserialize)]
pub struct Permissions {
    #[serde(default)]
    pub bidder_id: Vec<BidderId>,
    #[serde(default)]
    pub can_create_bid: bool,
    #[serde(default)]
    pub can_query_bid: bool,
    #[serde(default)]
    pub can_update_bid: bool,
    #[serde(default)]
    pub can_read_bid: bool,
    #[serde(default)]
    pub can_view_products: bool,
    #[serde(default)]
    pub can_manage_products: bool,
    #[serde(default)]
    pub can_run_batch: bool,
}

impl Display for Permissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_html_form::to_string(self).unwrap())
    }
}

impl FromStr for Permissions {
    type Err = serde_html_form::de::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let deserializer = serde_html_form::Deserializer::new(form_urlencoded::parse(s.as_bytes()));
        Self::deserialize(deserializer)
    }
}
