use axum_test::{TestServer, TestServerConfig};
use fts_axum::{config::AxumConfig, router};
use fts_sqlite::{Db, config::SqliteConfig, types::DateTime};
use hurl::runner::{self, VariableSet};
use hurl::runner::{RunnerOptionsBuilder, Value};
use hurl::util::logger::LoggerOptionsBuilder;
use hurl_core::input::Input;
use rstest::*;
use std::path::PathBuf;

mod app;
use app::TestApp;

// Important:
// Without a multithreaded runtime, the tests cannot make progress
#[rstest]
#[test_log::test(tokio::test(flavor = "multi_thread", worker_threads = 2))]
async fn test_api(#[files("tests/api/**/*.hurl")] test: PathBuf) {
    let app = {
        let config = SqliteConfig::default();
        let now = DateTime::from(time::OffsetDateTime::now_utc());
        let db = Db::open(&config, now).await.unwrap();
        TestApp(db)
    };

    let router = router(app, AxumConfig::default());

    // Run the test server on a local TCP port
    let server = TestServer::new_with_config(
        router,
        TestServerConfig {
            transport: Some(axum_test::Transport::HttpRandomPort),
            ..Default::default()
        },
    )
    .unwrap();

    let address = server.server_address().unwrap();
    let baseurl = address.as_str().strip_suffix("/").unwrap();

    // Read hurl file
    let content = std::fs::read_to_string(&test).unwrap();

    // Set the baseurl variable
    let mut variables = VariableSet::new();
    variables
        .insert("baseurl".to_string(), Value::String(baseurl.to_string()))
        .unwrap();

    // Run it
    let runner_opts = RunnerOptionsBuilder::new().follow_location(true).build();
    let logger_opts = LoggerOptionsBuilder::new().build();
    let result = runner::run(
        &content,
        test.to_str().map(|s| Input::new(s)).as_ref(),
        &runner_opts,
        &variables,
        &logger_opts,
    )
    .unwrap();

    assert!(result.success);
}
