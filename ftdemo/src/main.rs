use ftdemo::{AppConfig, Cli, Commands, impls::DemoApp};
use fts_axum::{schema, start_server};
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

    match cli.command {
        Commands::Schema { output } => {
            let schema = schema::<DemoApp>();
            serde_json::to_writer_pretty(output.write()?, &schema)?;
        }
        Commands::Serve { config, secret } => {
            let key = HS256Key::from_bytes(secret.as_bytes());

            // Create config with proper layering of CLI args
            let AppConfig {
                server,
                database,
                schedule,
            } = AppConfig::load(config)?;

            // Open database with config
            let db = Db::open(&database).await?;
            let db2 = db.clone();
            let app = DemoApp { db, key };

            // We always run the server task.
            let server_task = tokio::spawn(async move { start_server(server, app).await });

            // However, we may or may not also run a scheduled batch task
            if schedule.every.is_some() {
                let solver_task = tokio::spawn(async move {
                    let batch_config = schedule
                        .batch_config
                        .clone()
                        .expect("config must be provided");
                    let f = async move |now: OffsetDateTime| {
                        let batch = db2
                            .run_batch(
                                now.into(),
                                batch_config.clone(),
                                ClarabelSolver::default(),
                                (),
                            )
                            .await;
                        match batch {
                            Ok(Ok(expires)) => Ok(expires),
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
        }
    }

    Ok(())
}
