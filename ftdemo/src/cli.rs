//! Command-line interface definition and parsing.
//!
//! This module defines the command-line arguments accepted by the application
//! and provides parsing functionality using the clap crate.

use clap::{Parser, Subcommand};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write, stdin, stdout},
    path::PathBuf,
    str::FromStr,
};

/// Command-line arguments for the flow trading application.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// The various subcommands of this program
    #[command(subcommand)]
    pub command: Commands,
}

/// The action to take. Currently, run a server or print the OpenAPI schema
#[derive(Subcommand)]
pub enum Commands {
    /// Run an API server with the specified config and JWT secret
    Serve {
        /// Path to configuration file.
        #[arg(short, long, env = "APP_CONFIG")]
        config: Option<PathBuf>,

        /// The HMAC secret for verification of JWT claims.
        #[arg(short, long, env = "APP_SECRET")]
        secret: String,
    },

    /// Output the OpenAPI schema for the API
    Schema {
        /// The location to write the OpenAPI schema
        #[arg(short, long, default_value = "-")]
        output: PathOrStd,
    },
}

impl Cli {
    /// Parse command-line arguments.
    ///
    /// This method parses the command-line arguments according to the defined
    /// structure, including validation and help text generation.
    pub fn import() -> Result<Self, clap::Error> {
        Self::try_parse()
    }
}

#[derive(Clone)]
pub enum PathOrStd {
    Path(PathBuf),
    Std,
}

impl FromStr for PathOrStd {
    type Err = <PathBuf as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "-" {
            Ok(Self::Std)
        } else {
            Ok(Self::Path(s.parse()?))
        }
    }
}

impl PathOrStd {
    pub fn read(&self) -> anyhow::Result<Box<dyn Read>> {
        match self {
            PathOrStd::Path(path) => Ok(Box::new(BufReader::new(File::open(path)?))),
            PathOrStd::Std => Ok(Box::new(stdin().lock())),
        }
    }

    pub fn write(&self) -> anyhow::Result<Box<dyn Write>> {
        match self {
            PathOrStd::Path(path) => Ok(Box::new(BufWriter::new(File::create(path)?))),
            PathOrStd::Std => Ok(Box::new(stdout().lock())),
        }
    }
}
