use axum_test::TestServer;
use fts_core::{
    models::{AuctionOutcome, AuthId, AuthRecord, BidderId, DateTimeRangeResponse, ProductId},
    ports::MarketRepository,
};
use fts_server::{CustomJWTClaims, router, state};
use jwt_simple::prelude::{Claims, Duration, HS256Key, MACLike};
use rstest::rstest;
use rstest_reuse::apply;
use serde_json::json;
use std::any::Any;
use std::future::Future;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

mod backends;
use backends::*;

#[apply(all_backends)]
#[rstest]
async fn roundtrip(#[case] backend: impl Future<Output = (impl MarketRepository, Box<dyn Any>)>) {
    let (db, _container) = backend.await;

    // Create the server
    let app = {
        let state = state("secret", db);
        router(state.0)
    };

    // Create some JWT tokens
    let admin_token = {
        let key = HS256Key::from_bytes("secret".as_bytes());
        let account: BidderId = Uuid::new_v4().into();
        let account_str = account.to_string();
        let claims = Claims::with_custom_claims(
            CustomJWTClaims { admin: true },
            Duration::from_days(365 * 20 + 5),
        )
        .with_subject(&account_str);
        key.authenticate(claims).unwrap()
    };

    let account_tokens = (0..3)
        .map(|_| {
            let key = HS256Key::from_bytes("secret".as_bytes());
            let account: BidderId = Uuid::new_v4().into();
            let account_str = account.to_string();
            let claims =
                Claims::create(Duration::from_days(365 * 20 + 5)).with_subject(&account_str);
            (account, key.authenticate(claims).unwrap())
        })
        .collect::<Vec<_>>();

    let server = TestServer::new(app).unwrap();

    // Define the products using the admin token
    let (apple, banana, carrot, daikon) = {
        let (from, thru) = {
            let from = OffsetDateTime::now_utc();
            let thru = from + std::time::Duration::from_secs(60);
            (
                from.format(&Rfc3339).unwrap(),
                thru.format(&Rfc3339).unwrap(),
            )
        };

        let produce = server
            .post("/admin/products")
            .authorization_bearer(&admin_token)
            .json(&json!([
                { "kind": "APPLE", "from": from, "thru": thru },
                { "kind": "BANANA", "from": from, "thru": thru },
                { "kind": "CARROT", "from": from, "thru": thru },
                { "kind": "DAIKON", "from": from, "thru": thru },
            ]))
            .await
            .json::<Vec<ProductId>>();

        (produce[0], produce[1], produce[2], produce[3])
    };

    // account_tokens[0] will be a FRUIT seller
    let fruit_id = {
        // Step 1: Create a portfolio and authorization for it
        let auth_id: AuthId = {
            let response = server
                .post("/v0/auths")
                .authorization_bearer(&account_tokens[0].1)
                .json(&json!({
                    "portfolio": {
                        apple: 1.0,
                        banana: 1.0
                    },
                    "data": {
                        "min_rate": -10.0,
                        "max_rate": 0.0,
                        "min_trade": serde_json::Value::Null,
                        "max_trade": serde_json::Value::Null,
                    }
                }))
                .await
                .json::<AuthRecord>();

            response.auth_id
        };

        // Step 2: Create a cost and submit data for it
        server
            .post("/v0/costs")
            .authorization_bearer(&account_tokens[0].1)
            .json(&json!({
                "group": {
                    auth_id: 1.0
                },
                "data": [
                    { "rate": -10.0, "price": 10.0 },
                    { "rate": 0.0, "price": 5.0 }
                ]
            }))
            .await;

        auth_id
    };

    // we do the same thing again, with i=1 a VEGGIE seller and i=2 a buyer of any produce
    let veggie_id = {
        // Step 1: Create a portfolio and authorization for it
        let auth_id: AuthId = {
            let response = server
                .post("/v0/auths")
                .authorization_bearer(&account_tokens[1].1)
                .json(&json!({
                    "portfolio": {
                        carrot: 1.0,
                        daikon: 1.0
                    },
                    "data": {
                        "min_rate": -10.0,
                        "max_rate": 0.0,
                        "min_trade": serde_json::Value::Null,
                        "max_trade": serde_json::Value::Null,
                    }
                }))
                .await
                .json::<AuthRecord>();

            response.auth_id
        };

        // Step 2: Create a cost and submit data for it
        server
            .post("/v0/costs")
            .authorization_bearer(&account_tokens[1].1)
            .json(&json!({
                "group": {
                    auth_id: 1.0
                },
                "data": [
                    { "rate": -10.0, "price": 10.0 },
                    { "rate": 0.0, "price": 5.0 }
                ]
            }))
            .await;

        auth_id
    };

    let produce_id = {
        // Step 1: Create a portfolio and authorization for it
        let auth_id: AuthId = {
            let response = server
                .post("/v0/auths")
                .authorization_bearer(&account_tokens[2].1)
                .json(&json!({
                    "portfolio": {
                        apple: 1.0,
                        banana: 1.0,
                        carrot: 1.0,
                        daikon: 1.0,
                    },
                    "data":{
                        "min_rate": 0.0,
                        "max_rate": 10.0,
                        "min_trade": serde_json::Value::Null,
                        "max_trade": serde_json::Value::Null,
                    }
                }))
                .await
                .json::<AuthRecord>();

            response.auth_id
        };

        // Step 2: Create a cost and submit data for it
        server
            .post("/v0/costs")
            .authorization_bearer(&account_tokens[2].1)
            .json(&json!({
                "group": {
                    auth_id: 1.0
                },
                "data": [
                    { "rate": 0.0, "price": 30.0 },
                    { "rate": 10.0, "price": 0.0 }
                ]
            }))
            .await;

        auth_id
    };

    // In theory we have an auction!
    // We schedule, and then solve, the auction.
    let (from, thru) = {
        let from = OffsetDateTime::now_utc() + std::time::Duration::from_secs(60);
        let thru = from + std::time::Duration::from_secs(60 * 60);
        (from, thru)
    };
    server
        .post("/admin/auctions/solve")
        .authorization_bearer(&admin_token)
        .json(&json!({
            "from": from.format(&Rfc3339).unwrap(),
            "thru": thru.format(&Rfc3339).unwrap(),
        }))
        .await;

    // Wait 1 second for solver to work itself through
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Now let's see what the results are!
    {
        // { "results": [...], "more": null }
        let response = server
            .get(&format!("/v0/auths/{}/outcomes", fruit_id))
            .authorization_bearer(&account_tokens[0].1)
            .await
            .json::<DateTimeRangeResponse<AuctionOutcome<()>>>();

        assert_eq!(response.results.len(), 1);
        assert_eq!((response.results[0].outcome.trade * 100.0).round(), -500.0);
    }
    {
        // { "results": [...], "more": null }
        let response = server
            .get(&format!("/v0/auths/{}/outcomes", veggie_id))
            .authorization_bearer(&account_tokens[1].1)
            .await
            .json::<DateTimeRangeResponse<AuctionOutcome<()>>>();

        assert_eq!(response.results.len(), 1);
        assert_eq!((response.results[0].outcome.trade * 100.0).round(), -500.0);
    }
    {
        // { "results": [...], "more": null }
        let response = server
            .get(&format!("/v0/auths/{}/outcomes", produce_id))
            .authorization_bearer(&account_tokens[2].1)
            .await
            .json::<DateTimeRangeResponse<AuctionOutcome<()>>>();

        assert_eq!(response.results.len(), 1);
        assert_eq!((response.results[0].outcome.trade * 100.0).round(), 500.0);
    }

    // What about the prices?
    for product in [apple, banana, carrot, daikon] {
        // No bearer token necessary!
        // { "results": [...], "more": null }
        let response = server
            .get(&format!("/v0/products/{}/outcomes", product))
            .await
            .json::<DateTimeRangeResponse<AuctionOutcome<()>>>();

        assert_eq!(response.results.len(), 1);

        let price = (response.results[0].outcome.price * 100.0).round();
        assert_eq!(price, 375.0);
    }
}
