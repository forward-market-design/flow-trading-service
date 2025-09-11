use std::time::Duration;

/// Any configuration related to the batch
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct BatchConfig {
    /// The reference time unit, in milliseconds
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "humantime_serde::deserialize",
            serialize_with = "humantime_serde::serialize"
        )
    )]
    pub time_unit: Duration,
}
