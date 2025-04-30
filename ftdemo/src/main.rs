use clap::{Args, Parser};
use fts_core::models::Config;
use fts_sqlite::db::{self, Database};
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[cfg(feature = "testmode")]
use fts_server::generate_jwt;

#[tokio::main]
async fn main() -> Result<(), db::Error> {
    // By convention, we leverage `tracing` to instrument and log various
    // operations throughout this project.
    // Accordingly, we likely want to subscribe to these events so we can
    // write them to stdio and possibly some durable location.
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    // We will need to collect whatever data is necessary to configure the API
    // server. The `Args` struct, defined after this function, does this job for us.
    let args = Cli::import();

    match args {
        Ok(args) => {
            // We check (and store) the configuration into our database,
            // so we're not in an inconsistent state.
            let database = Database::open(
                args.database.as_ref(),
                Some(Config {
                    trade_rate: args.trade_rate.into(),
                }),
            )?;

            // If the backend is launched in testing mode, we print the credentials for an admin and as many random users as requested
            #[cfg(feature = "testmode")]
            if let Some(n) = args.test {
                let (jwt, id) = generate_jwt(&args.api_secret, 1, true).unwrap();
                println!("Admin:\n\tUUID: {}\n\tJWT: {}\n", id, jwt);
                for i in 1..=n {
                    let (jwt, id) = generate_jwt(&args.api_secret, 1, false).unwrap();
                    println!("User({}):\n\tUUID: {}\n\tJWT: {}\n", i, id, jwt);
                }
            }

            // Finally, we provide this data to the API module, which creates an HTTP
            // server on the specified port.
            fts_server::start(args.api_port, args.api_secret, database).await;

            // TODO: craft an f(from, thru) function that triggers the solve.
            // args.schedule.schedule(f)
        }
        Err(e) => {
            let _ = e.print();
        }
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The port to listen on
    #[arg(long, default_value_t = 8080, env = "API_PORT")]
    pub api_port: u16,

    /// The HMAC-secret for verification of JWT claims
    #[arg(long, env = "API_SECRET")]
    pub api_secret: String,

    /// Set this flag to create N random bidders and print their tokens, for use in external tools.
    #[cfg(feature = "testmode")]
    #[arg(long)]
    pub test: Option<u32>,

    /// The location of the database (if omitted, use an in-memory db)
    #[arg(long, env = "DATABASE")]
    pub database: Option<std::path::PathBuf>,

    /// The time unit of rate data
    #[arg(long, env = "TRADE_RATE")]
    pub trade_rate: humantime::Duration,

    /// The args related to scheduling batch execution
    #[command(flatten)]
    pub schedule: Scheduler,
}

impl Cli {
    pub fn import() -> Result<Self, clap::Error> {
        // Attempt to load a .env file, but don't sweat it if one is not found.
        let _ = dotenvy::dotenv();
        Self::try_parse()
    }
}

#[derive(Args, Clone, Debug)]
pub struct Scheduler {
    /// An RFC3339 timestamp to start the auction schedule from (if omitted or empty, defaults to now)
    #[arg(long, requires = "schedule", env = "SCHEDULE_FROM", value_parser = parse_timestamp)]
    pub schedule_from: Option<time::OffsetDateTime>,
    /// How often to execute an auction
    #[arg(long, group = "schedule", env = "SCHEDULE_EVERY")]
    pub schedule_every: Option<humantime::Duration>,
}

impl Scheduler {
    pub async fn schedule(&self, f: impl Fn(time::OffsetDateTime, time::OffsetDateTime) -> ()) {
        // extract the duration into a std::time::Duration
        let delta = if let Some(delta) = self.schedule_every {
            <humantime::Duration as Into<std::time::Duration>>::into(delta)
        } else {
            return;
        };

        let now = time::OffsetDateTime::now_utc();

        // adjust the anchor time to be >= now
        let mut anchor = if let Some(mut dt) = self.schedule_from {
            if dt < now {
                let x = ((now - dt) / delta).ceil() as u32;
                dt += delta * x;
            }
            dt
        } else {
            now
        };

        // now we align the clocks as best we can
        {
            let sleepy: std::time::Duration = (anchor - now)
                .try_into()
                .expect("anchor too far in the future");

            tokio::time::sleep(sleepy).await;
        };

        // Finally, we can loop over a timer
        let mut interval = tokio::time::interval(delta);
        loop {
            interval.tick().await;
            let next = anchor + delta;
            f(anchor, next);
            anchor = next;
        }
    }
}

fn parse_timestamp(timestamp: &str) -> Result<time::OffsetDateTime, time::error::Parse> {
    time::OffsetDateTime::parse(timestamp, &time::format_description::well_known::Rfc3339)
}
