use clap::Args;
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write, stdin, stdout},
    path::PathBuf,
    str::FromStr,
};

// Most (all, presently) subcommands have a notion of input and output.
// This struct standardizes their implementation.
#[derive(Args)]
pub struct IOArgs {
    /// The auction JSON file ("-" implies stdin)
    #[arg(value_parser = clap::value_parser!(PathOrStd))]
    input: PathOrStd,

    /// The output file ("-" implies stdout)
    #[arg(short, long, default_value = "-", value_parser = clap::value_parser!(PathOrStd))]
    output: PathOrStd,
}

impl IOArgs {
    pub fn read(&self) -> anyhow::Result<Box<dyn Read>> {
        match &self.input {
            PathOrStd::Path(path) => Ok(Box::new(BufReader::new(File::open(path)?))),
            PathOrStd::Std => Ok(Box::new(stdin().lock())),
        }
    }

    pub fn write(&self) -> anyhow::Result<Box<dyn Write>> {
        match &self.output {
            PathOrStd::Path(path) => Ok(Box::new(BufWriter::new(File::create(path)?))),
            PathOrStd::Std => Ok(Box::new(stdout().lock())),
        }
    }

    pub fn extension(&self) -> Option<&str> {
        match &self.output {
            PathOrStd::Path(path) => path.extension(),
            PathOrStd::Std => None,
        }
        .and_then(|ext| ext.to_str())
    }
}

#[derive(Clone)]
enum PathOrStd {
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
