use clap::{Args, Parser, Subcommand, ValueEnum};
use fts_solver::{
    Auction, AuctionOutcome, Solver as _, Submission,
    clarabel::ClarabelSolver,
    export::{export_lp, export_mps},
    io::{AuctionDto, BidderId, PortfolioId, ProductId},
    osqp::OsqpSolver,
};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write, stdin, stdout},
    ops::Deref,
    path::PathBuf,
};

// The top-level arguments -- presently just which subcommand to execute
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct BaseArgs {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Solve the auction and report the solution
    Solve {
        #[command(flatten)]
        io: IOArgs,

        /// Request a specific QP solver
        #[arg(short, long, default_value = "clarabel")]
        lib: SolverLib,
    },

    /// Construct the flow trading quadratic program and export to a standard format
    Export {
        #[command(flatten)]
        io: IOArgs,

        /// The file format to use (if omitted, will infer based on filename)
        #[arg(short, long)]
        format: Option<ExportFormat>,
    },
}

// Most (all, presently) subcommands have a notion of input and output.
// This struct standardizes their implementation.
#[derive(Args)]
struct IOArgs {
    /// The auction JSON file (defaults to stdin if omitted)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// The output file (defaults to stdout if omitted)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

impl IOArgs {
    fn read(&self) -> anyhow::Result<Box<dyn Read>> {
        if let Some(path) = &self.input {
            Ok(Box::new(BufReader::new(File::open(path)?)))
        } else {
            Ok(Box::new(stdin().lock()))
        }
    }

    fn write(&self) -> anyhow::Result<Box<dyn Write>> {
        if let Some(path) = &self.output {
            Ok(Box::new(BufWriter::new(File::create(path)?)))
        } else {
            Ok(Box::new(stdout().lock()))
        }
    }

    fn extension(&self) -> Option<&str> {
        self.output
            .as_ref()
            .and_then(|path| path.extension())
            .and_then(|ext| ext.to_str())
    }
}

// This explicitly articulates the available solvers for the `solve` subcommand
#[derive(Clone, Copy, ValueEnum)]
enum SolverLib {
    Clarabel,
    Osqp,
}

// Conveniently, we can use the same enum to handle the particulars of calling into
// the various solver implementations
impl SolverLib {
    fn solve<T>(&self, auction: &T) -> AuctionOutcome<BidderId, PortfolioId, ProductId>
    where
        for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<PortfolioId, ProductId>)>,
    {
        match self {
            SolverLib::Clarabel => {
                let solver = ClarabelSolver::default();
                solver.solve(auction)
            }
            SolverLib::Osqp => {
                let solver = OsqpSolver::default();
                solver.solve(auction)
            }
        }
    }
}

// Same story here with the ExportFormat enum, as with the SolverLib enum
#[derive(Clone, Copy, ValueEnum)]
enum ExportFormat {
    Mps,
    Lp,
}

impl ExportFormat {
    fn export<T, W>(&self, auction: &T, buffer: &mut W) -> anyhow::Result<()>
    where
        for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<PortfolioId, ProductId>)>,
        W: Write,
    {
        match self {
            Self::Mps => export_mps(auction, buffer)?,
            Self::Lp => export_lp(auction, buffer)?,
        };
        Ok(())
    }
}

pub fn main() -> anyhow::Result<()> {
    let args = BaseArgs::parse();

    match args.command {
        Commands::Solve { io, lib } => {
            let input = io.read()?;
            let auction: Auction<_, _, _> =
                serde_json::from_reader::<_, AuctionDto>(input)?.try_into()?;
            let results = lib.solve(auction.deref());
            let output = io.write()?;
            serde_json::to_writer_pretty(output, &results)?;
        }
        Commands::Export { io, format } => {
            let input = io.read()?;
            let auction: Auction<_, _, _> =
                serde_json::from_reader::<_, AuctionDto>(input)?.try_into()?;
            let format = format
                .or_else(|| match io.extension() {
                    Some("mps") => Some(ExportFormat::Mps),
                    Some("lp") => Some(ExportFormat::Lp),
                    _ => None,
                })
                .ok_or(CliError::ExportExtension)?;

            let mut output = io.write()?;
            format.export(auction.deref(), &mut output)?;
        }
    }

    Ok(())
}

#[derive(thiserror::Error, Debug)]
enum CliError {
    #[error("Unsupported export format")]
    ExportExtension,
}
