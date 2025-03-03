use axum::http::StatusCode;
use axum_test::TestServer;
use fts_core::{
    models::{
        AuthId, AuthRecord, BidderId, CostHistoryRecord, CostRecord, DateTimeRangeResponse,
        ProductId, SubmissionRecord,
    },
    ports::MarketRepository,
};
use fts_server::{CustomJWTClaims, router, state};
use jwt_simple::prelude::{Claims, Duration, HS256Key, MACLike};
use rstest::rstest;
use rstest_reuse::apply;
use serde_json::json;
use std::{any::Any, future::Future};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

mod backends;
use backends::*;

#[apply(all_backends)]
#[rstest]
async fn crud(#[case] backend: impl Future<Output = (impl MarketRepository, Box<dyn Any>)>) {
    // Create the server
    let (db, _container) = backend.await;

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

    // Create the client JWT token
    let (account, token) = {
        let key = HS256Key::from_bytes("secret".as_bytes());
        let account: BidderId = Uuid::new_v4().into();
        let account_str = account.to_string();
        let claims = Claims::create(Duration::from_days(365 * 20 + 5)).with_subject(&account_str);
        (account, key.authenticate(claims).unwrap())
    };

    let server = TestServer::new(app).unwrap();

    // Define the products using the admin token
    let (apple, banana) = {
        let (from, thru) = {
            let from = OffsetDateTime::now_utc();
            let thru = from + std::time::Duration::from_secs(60);
            (
                from.format(&Rfc3339).unwrap(),
                thru.format(&Rfc3339).unwrap(),
            )
        };

        let response = server
            .post("/admin/products")
            .authorization_bearer(&admin_token)
            .json(&json!([
                { "kind": "APPLE", "from": from, "thru": thru },
                { "kind": "BANANA", "from": from, "thru": thru }
            ]))
            .await;

        let fruits = response.json::<Vec<ProductId>>();

        (fruits[0], fruits[1])
    };

    {
        // Table stakes: fresh server, new bidder, make sure we have no existing orders
        let submission = server
            .get(&format!("/v0/submissions/{}", account))
            .authorization_bearer(&token)
            .await
            .json::<SubmissionRecord>();
        assert_eq!(submission.costs.len(), 0);

        // For fun, send a request without the token, see what happens
        assert_eq!(
            server
                .get(&format!("/v0/submissions/{}", account))
                .await
                .status_code(),
            StatusCode::UNAUTHORIZED
        );
    }

    let auth_id: AuthId = {
        let res = server
            .post("/v0/auths")
            .authorization_bearer(&token)
            .json(&json!({
                "portfolio": {
                    apple: 1.0,
                    banana: 1.0
                },
                "data": {
                    "min_rate": serde_json::Value::Null,
                    "max_rate": serde_json::Value::Null,
                    "min_trade": serde_json::Value::Null,
                    "max_trade": serde_json::Value::Null,
                }
            }))
            .await;
        let response = res.json::<AuthRecord>();
        response.auth_id
    };

    {
        let response = server
            .get(&format!("/v0/submissions/{}", account))
            .authorization_bearer(&token)
            .await
            .json::<SubmissionRecord>();

        assert_eq!(response.auths.len(), 1);
    }

    {
        let res = server
            .put(&format!("/v0/submissions/{}", account))
            .authorization_bearer(&token)
            .json(&json!({
                "costs": [],
                "auths": [
                    {
                        "auth_id": auth_id,
                    }
                ]
            }))
            .await;
        let response = res.json::<SubmissionRecord>();
        assert_eq!(response.auths.len(), 1);
        assert_eq!(response.auths[0].auth_id, auth_id);
    }

    let (cost_id, version) = {
        let res = server
            .post("/v0/costs")
            .authorization_bearer(&token)
            .json(&json!({
                "group": {
                    auth_id: 1.0
                },
                "data": [
                    { "rate": -5.0, "price": 10.0 },
                    { "rate": 5.0, "price": 5.0 }
                ]
            }))
            .await;

        let response = res.json::<CostRecord>();

        (response.cost_id, response.version)
    };

    // Let's now try a few things. First, let's list our bids again:
    {
        let submission = server
            .get(&format!("/v0/submissions/{}", account))
            .authorization_bearer(&token)
            .await
            .json::<SubmissionRecord>();
        assert_eq!(submission.costs.len(), 1);
        assert_eq!(submission.costs[0].bidder_id, account);
        assert_eq!(submission.costs[0].cost_id, cost_id);
        assert_eq!(submission.costs[0].version, version);
    }

    // Let's also grab the most recent order and verify it is this one
    {
        let bid = server
            .get(&format!("/v0/costs/{}", cost_id))
            .authorization_bearer(&token)
            .await
            .json::<CostRecord>();
        assert_eq!(bid.version, version);
    };

    // We can also try to explicitly grab the order with its version
    {
        let version_iso = urlencoding::encode(&version.format(&Rfc3339).unwrap()).to_string();
        let costs = server
            .get(&format!(
                "/v0/costs/{}/history?before={}&after={}",
                cost_id, version_iso, version_iso
            ))
            .authorization_bearer(&token)
            .await;

        let costs = costs.json::<DateTimeRangeResponse<CostHistoryRecord>>();
        assert_eq!(costs.results.len(), 1);
        assert!(costs.more.is_none());
        assert!(costs.results[0].data.is_some());
        assert_eq!(costs.results[0].version, version);
    }

    // now let's patch the order (keeping the same portfolio)
    {
        let bid = server
            .put(&format!("/v0/costs/{}", cost_id))
            .authorization_bearer(&token)
            .json(&json!({"data": [
                { "rate": -5.0, "price": 15.0 },
                { "rate": 5.0, "price": 5.0 }
            ]}))
            .await;

        let cost = bid.json::<CostRecord>();
        assert_ne!(version, cost.version);
    }
    // test the deletion of the order and ensure the order list does not pick it up
    {
        let response = server
            .delete(&format!("/v0/costs/{}", cost_id))
            .authorization_bearer(&token)
            .await;

        assert_eq!(response.status_code(), StatusCode::OK);

        let submission = server
            .get(&format!("/v0/submissions/{}", account))
            .authorization_bearer(&token)
            .await
            .json::<SubmissionRecord>();

        assert_eq!(submission.costs.len(), 0);
    }
}
