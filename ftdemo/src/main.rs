use std::fs::File;

use ftdemo::{AppConfig, Cli, impls::DemoApp};
use fts_axum::{router, start_server};
use fts_core::ports::BatchRepository as _;
use fts_solver::clarabel::ClarabelSolver;
use fts_sqlite::Db;
use jwt_simple::prelude::HS256Key;
use time::OffsetDateTime;
use tokio::select;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // By convention, we leverage `tracing` to instrument and log various
    // operations throughout this project.
    // Accordingly, we likely want to subscribe to these events so we can
    // write them to stdio and possibly some durable location.
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse CLI args and extract the JWT key
    let cli = Cli::import()?;
    let key = HS256Key::from_bytes(cli.secret.as_bytes());

    // Create config with proper layering of CLI args
    let AppConfig {
        server,
        database,
        schedule,
    } = AppConfig::load(&cli)?;

    // Open database with config
    let db = Db::open(&database, OffsetDateTime::now_utc().into()).await?;
    let db2 = db.clone();
    let app = DemoApp { db, key };

    // If requested, dump the schema and exit.
    if let Some(path) = cli.schema {
        let schema = router(app, server).1;
        serde_json::to_writer_pretty(File::create(path)?, &schema)?;
        return Ok(());
    }

    // We always run the server task.
    let server_task = tokio::spawn(async move { start_server(server, app).await });

    // However, we may or may not also run a scheduled batch task
    if schedule.every.is_some() {
        let solver_task = tokio::spawn(async move {
            let f = async move |now: OffsetDateTime| {
                let batch = db2
                    .run_batch(now.into(), ClarabelSolver::default(), ())
                    .await;
                match batch {
                    Ok(Ok(())) => Ok(()),
                    Ok(Err(e)) => Err(anyhow::Error::new(e)),
                    Err(e) => Err(anyhow::Error::new(e)),
                }
            };
            schedule.schedule(f).await
        });

        select! {
            r = server_task => r??,
            r = solver_task => r??,
        }
    } else {
        // Otherwise, we just run the server task to completion
        server_task.await??;
    }

    Ok(())
}
