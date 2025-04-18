use clap::ValueEnum;
use fts_solver::{
    Submission,
    export::{export_lp, export_mps},
    io::{BidderId, PortfolioId, ProductId},
};
use std::{io::Write, str::FromStr};

// Same story here with the ExportFormat enum, as with the SolverLib enum
#[derive(Clone, Copy, ValueEnum)]
pub enum ExportFormat {
    Mps,
    Lp,
}

impl ExportFormat {
    pub fn export<T, W>(&self, auction: &T, buffer: &mut W) -> anyhow::Result<()>
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

impl FromStr for ExportFormat {
    type Err = ExportFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mps" | "MPS" => Ok(Self::Mps),
            "lp" | "LP" => Ok(Self::Lp),
            _ => Err(Self::Err::ExportExtension(s.to_owned())),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ExportFormatError {
    #[error("Unknown export format: {0}")]
    ExportExtension(String),
}
