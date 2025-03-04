use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::db;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "RawConfig", into = "RawConfig")]
pub struct Config {
    pub trade_rate: Duration,
}

impl Config {
    pub fn get(conn: &Connection) -> Result<Option<Self>, db::Error> {
        let response: Option<serde_json::Value> = conn
            .query_row("select data from config where id = 0 limit 1", (), |row| {
                row.get(0)
            })
            .optional()?;

        if let Some(config_data) = response {
            let config: Config = serde_json::from_value(config_data)?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    pub fn set(&self, conn: &Connection) -> Result<(), db::Error> {
        conn.execute("insert into config (id, data) values (0, ?1) on conflict (id) do update set data = excluded.data", (serde_json::to_value(self)?,))?;
        Ok(())
    }
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
