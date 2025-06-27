//! Command-line interface definition and parsing.
//!
//! This module defines the command-line arguments accepted by the application
//! and provides parsing functionality using the clap crate.

use clap::Parser;
use std::path::PathBuf;

/// Command-line arguments for the flow trading application.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Path to configuration file.
    #[arg(short, long, env = "APP_CONFIG")]
    pub config: Option<PathBuf>,

    /// The HMAC secret for verification of JWT claims.
    #[arg(short, long, env = "APP_SECRET")]
    pub secret: String,
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
