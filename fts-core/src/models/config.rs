use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Flow trading takes place within a context. This config describes this context.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "RawConfig", into = "RawConfig")]
pub struct Config {
    /// The rate to use for converting rates into batch quantities.
    pub trade_rate: Duration,
}

// To seamlessly (de)serialize, we create a "raw" version of of our struct
// that contains only primitive values. We tell Serde to use the raw version
// to handle (de)serialization, then call .into() to get our rich version.
// It's a little verbose, but fairly clean.
// Note that the u64<>u32 conversion is because JSON serialization does not
// support 64 bit integers.

#[derive(Serialize, Deserialize)]
pub struct RawConfig {
    pub trade_rate: u32,
}

impl From<RawConfig> for Config {
    fn from(value: RawConfig) -> Self {
        Self {
            trade_rate: Duration::from_secs(value.trade_rate as u64),
        }
    }
}

impl From<Config> for RawConfig {
    fn from(value: Config) -> Self {
        Self {
            trade_rate: value.trade_rate.as_secs() as u32,
        }
    }
}
