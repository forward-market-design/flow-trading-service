mod admin;
pub use admin::Admin;

mod bidder;
pub use bidder::Bidder;

mod jwt;
pub use jwt::{CustomJWTClaims, JWTVerifier};

mod now;
pub use now::Now;

mod pubsub;
pub use pubsub::*;

use jwt_simple::{
    claims::Claims,
    prelude::{Duration, HS256Key, MACLike},
};
use uuid::Uuid;

use fts_core::models::BidderId;

/// Generate a JWT token and account string
pub fn generate_jwt(
    raw_key: &str,
    duration_days: u64,
    is_admin: bool,
) -> Result<(String, String), jwt_simple::Error> {
    let key = HS256Key::from_bytes(raw_key.as_bytes());
    let account: BidderId = Uuid::new_v4().into();
    let account_str = account.to_string();
    let claims = Claims::with_custom_claims(
        CustomJWTClaims { admin: is_admin },
        Duration::from_days(duration_days),
    )
    .with_subject(&account_str);

    let token = key.authenticate(claims)?;
    Ok((token, account_str))
}
