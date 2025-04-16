use clap::{Parser, Subcommand, ValueEnum};
use fts_solver::{
    Auction, AuctionOutcome, Solver as _, Submission,
    clarabel::ClarabelSolver,
    export::{export_lp, export_mps},
    io::{AuctionDto, BidderId, PortfolioId, ProductId},
    osqp::OsqpSolver,
};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Write, stdin, stdout},
    ops::Deref,
    path::PathBuf,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// The auction JSON file (defaults to stdin if omitted)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// The output file (defaults to stdout if omitted)
    #[arg(short, long)]
    output: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Solve the auction and report the solution
    Solve {
        /// Request a specific QP solver
        #[arg(short, long, default_value = "clarabel")]
        lib: SolverLib,
    },

    /// Export the quadratic program without solving
    Export {
        /// The file format to use (if omitted, will infer based on filename)
        #[arg(short, long)]
        format: Option<ExportFormat>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum SolverLib {
    Clarabel,
    Osqp,
}

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

impl Args {
    fn read(&self) -> anyhow::Result<AuctionDto> {
        if let Some(path) = &self.input {
            let reader = BufReader::new(File::open(path)?);
            Ok(serde_json::from_reader(reader)?)
        } else {
            let reader = stdin().lock();
            Ok(serde_json::from_reader(reader)?)
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

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let raw = args.read()?;
    let input: Auction<_, _, _> = raw.try_into()?;

    match &args.command {
        Commands::Solve { lib } => {
            let output = lib.solve(input.deref());
            let writer = args.write()?;
            serde_json::to_writer_pretty(writer, &output)?;
        }
        Commands::Export { format } => {
            let format = format
                .or_else(|| match args.extension() {
                    Some("mps") => Some(ExportFormat::Mps),
                    Some("lp") => Some(ExportFormat::Lp),
                    _ => None,
                })
                .ok_or(CliError::ExportExtension)?;

            let mut writer = args.write()?;
            format.export(input.deref(), &mut writer)?;
        }
    }

    Ok(())
}

#[derive(thiserror::Error, Debug)]
enum CliError {
    #[error("Unsupported export format")]
    ExportExtension,
}
