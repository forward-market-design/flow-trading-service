use clap::{Parser, ValueEnum};
use fts_solver::{
    Auction, AuctionOutcome, Solver as _, Submission,
    clarabel::ClarabelSolver,
    io::{AuctionDto, BidderId, PortfolioId, ProductId},
    osqp::OsqpSolver,
};
use std::{
    fmt::Display,
    fs::File,
    io::{BufReader, BufWriter, stdin, stdout},
    ops::Deref,
    path::PathBuf,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// The JSON file to read (defaults to stdin if omitted)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// The JSON file to write (defaults to stdout if omitted)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// The underlying solver implementation to use
    #[arg(short, long, default_value_t = SolverFlag::Clarabel)]
    solver: SolverFlag,
}

#[derive(Clone, ValueEnum)]
enum SolverFlag {
    Clarabel,
    Osqp,
}

impl SolverFlag {
    fn solve<T>(&self, auction: &T) -> AuctionOutcome<BidderId, PortfolioId, ProductId>
    where
        for<'t> &'t T: IntoIterator<Item = (&'t BidderId, &'t Submission<PortfolioId, ProductId>)>,
    {
        match self {
            SolverFlag::Clarabel => {
                let solver = ClarabelSolver::default();
                solver.solve(auction)
            }
            SolverFlag::Osqp => {
                let solver = OsqpSolver::default();
                solver.solve(auction)
            }
        }
    }
}

impl Display for SolverFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (match self {
            Self::Clarabel => "clarabel",
            Self::Osqp => "osqp",
        })
        .fmt(f)
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

    fn write(
        &self,
        outcome: &AuctionOutcome<BidderId, PortfolioId, ProductId>,
    ) -> anyhow::Result<()> {
        if let Some(path) = &self.output {
            let writer = BufWriter::new(File::create(path)?);
            serde_json::to_writer_pretty(writer, outcome)?;
        } else {
            let writer = stdout().lock();
            serde_json::to_writer_pretty(writer, outcome)?;
        }
        Ok(())
    }
}

pub fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let raw = args.read()?;
    let input: Auction<_, _, _> = raw.try_into()?;

    let output = args.solver.solve(input.deref());
    args.write(&output)?;

    Ok(())
}
