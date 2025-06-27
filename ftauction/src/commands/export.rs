use clap::ValueEnum;
use fts_solver::io::Auction;
use std::{io::Write, str::FromStr};

// Same story here with the ExportFormat enum, as with the SolverLib enum
#[derive(Clone, Copy, ValueEnum)]
pub enum ExportFormat {
    Mps,
    Lp,
}

impl ExportFormat {
    pub fn export<W: Write>(&self, auction: Auction, buffer: &mut W) -> anyhow::Result<()> {
        match self {
            Self::Mps => auction.export_mps(buffer)?,
            Self::Lp => auction.export_lp(buffer)?,
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
