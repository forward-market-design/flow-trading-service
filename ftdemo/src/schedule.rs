//! Scheduler for running periodic batch auctions.
//!
//! This module provides functionality to schedule and execute batch auctions at regular intervals.
//! The scheduler can be configured with a start time and execution frequency, and will automatically
//! align execution times with the configured schedule.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tracing::{Instrument as _, Level, event, span};

/// Configuration for scheduling periodic batch auctions.
///
/// The scheduler allows configuring when to start executing auctions and how frequently
/// to run them. It handles clock alignment to ensure auctions run at predictable intervals.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Scheduler {
    /// An RFC3339 timestamp to start the auction schedule from (if omitted or empty, defaults to now)
    #[serde(with = "time::serde::rfc3339::option")]
    pub from: Option<time::OffsetDateTime>,
    /// How often to execute an auction
    #[serde(with = "humantime_serde::option")]
    pub every: Option<Duration>,
}

impl Scheduler {
    /// Schedule and execute a function at regular intervals.
    ///
    /// This method will:
    /// 1. Calculate the next execution time based on the configured schedule
    /// 2. Wait until that time
    /// 3. Execute the provided function repeatedly at the configured interval
    ///
    /// # Arguments
    ///
    /// * `f` - An async function that takes a timestamp and returns a Result
    ///
    /// # Returns
    ///
    /// * `Ok(())` if scheduling is disabled (no interval configured)
    /// * `Err(E)` if the scheduled function returns an error
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use time::OffsetDateTime;
    /// use ftdemo::Scheduler;
    ///
    /// # fn main() -> Result<(), String> {
    /// let scheduler = Scheduler {
    ///     from: Some(OffsetDateTime::now_utc()),
    ///     every: Some(Duration::from_secs(3600)), // Every hour
    /// };
    ///
    /// # tokio_test::block_on(async {
    /// scheduler.schedule(|timestamp| async move {
    ///     println!("Running batch at {}", timestamp);
    ///     Ok::<(), String>(())
    /// }).await?;
    /// # Ok(())
    /// # })
    /// # }
    /// ```
    pub async fn schedule<T, E>(
        &self,
        f: impl AsyncFn(OffsetDateTime) -> Result<T, E>,
    ) -> Result<(), E> {
        // extract the duration or return immediately
        let Some(delta) = self.every else {
            return Ok(());
        };

        let now = time::OffsetDateTime::now_utc();

        // adjust the anchor time to be >= now
        let mut anchor = if let Some(mut from) = self.from {
            if from < now {
                let x = ((now - from) / delta).ceil() as u32;
                from += delta * x;
            }
            from
        } else {
            now
        };

        // now we align the clocks as best we can
        {
            let sleepy: std::time::Duration = (anchor - now)
                .try_into()
                .expect("anchor too far in the future");

            tokio::time::sleep(sleepy).await;
        };

        // Finally, we can loop over a timer
        let mut interval = tokio::time::interval(delta);

        loop {
            interval.tick().await;

            let span = span!(Level::INFO, "running scheduled auction");
            async {
                event!(Level::INFO, batch_time = anchor.format(&Rfc3339).unwrap(),);
                f(anchor).await
            }
            .instrument(span)
            .await?;

            anchor += delta;
        }
    }
}
