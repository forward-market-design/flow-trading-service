use clap::Parser;
use fts_demo::{
    Config,
    db::{self, Database},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "testmode")]
use marketplace::utils::generate_jwt;

#[tokio::main]
async fn main() -> Result<(), db::Error> {
    // By convention, we leverage `tracing` to instrument and log various
    // operations in our `simple_` and `marketplace_api` implementations.
    // Accordingly, we likely want to subscribe to these events so we can
    // write them to stdio and possibly some durable location.
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    // We will need to collect whatever data is necessary to configure the API
    // server and the orderbook, directory, and ledger implementations. The `Args`
    // struct, defined after this function, does this job for us.
    let args = Args::import();

    match args {
        Ok(args) => {
            // We now need to configure the OrderBook implementation.
            let database = Database::open(
                args.database.as_ref(),
                Some(&Config {
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
        }
        Err(e) => {
            let _ = e.print();
        }
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
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

    /// The location of the orderbook database (if omitted, use an in-memory db)
    #[arg(long, env = "DATABASE")]
    pub database: Option<std::path::PathBuf>,

    /// The duration of time rates are specified with respect to
    #[arg(long, env = "TRADE_RATE")]
    pub trade_rate: humantime::Duration,
}

impl Args {
    pub fn import() -> Result<Self, clap::Error> {
        // Attempt to load a .env file, but don't sweat it if one is not found.
        let _ = dotenvy::dotenv();
        Self::try_parse()
    }
}
