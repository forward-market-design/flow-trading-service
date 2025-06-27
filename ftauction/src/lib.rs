use clap::Parser;
use fts_solver::io::Auction;

mod io;
pub use io::*;

mod commands;
pub use commands::*;

// The top-level arguments -- presently just which subcommand to execute
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct BaseArgs {
    #[command(subcommand)]
    pub command: Commands,
}

impl BaseArgs {
    pub async fn evaluate(self) -> anyhow::Result<()> {
        match self.command {
            Commands::Solve { io, lib } => {
                let input = io.read()?;
                let auction = serde_json::from_reader::<_, Auction>(input)?;
                let results = lib.solve(auction).await;
                let output = io.write()?;
                serde_json::to_writer_pretty(output, &results)?;
            }
            Commands::Export { io, format } => {
                let input = io.read()?;
                let auction = serde_json::from_reader::<_, Auction>(input)?;

                let format = if let Some(format) = format {
                    format
                } else if let Some(ext) = io.extension() {
                    ext.parse()?
                } else {
                    return Err(CliError::ExportInference)?;
                };

                let mut output = io.write()?;
                format.export(auction, &mut output)?;
            }
        }

        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error("Unable to infer export format, please specify a valid format")]
    ExportInference,
}
