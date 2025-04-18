use super::IOArgs;
use clap::Subcommand;

mod export;
mod solve;

#[derive(Subcommand)]
pub enum Commands {
    /// Solve the auction and report the solution
    Solve {
        #[command(flatten)]
        io: IOArgs,

        /// Request a specific QP solver
        #[arg(short, long, default_value = "clarabel")]
        lib: solve::SolverLib,
    },

    /// Construct the flow trading quadratic program and export to a standard format
    Export {
        #[command(flatten)]
        io: IOArgs,

        /// The file format to use (if omitted, will infer based on filename)
        #[arg(short, long)]
        format: Option<export::ExportFormat>,
    },
}
