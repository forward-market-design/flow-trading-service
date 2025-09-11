use std::time::Duration;

/// Every batch executes with some context, e.g. for bidding rates what time
/// unit the rate is defined with respect to?
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct BatchConfig {
    /// The reference time unit
    #[cfg_attr(
        feature = "serde",
        serde(
            deserialize_with = "humantime_serde::deserialize",
            serialize_with = "humantime_serde::serialize"
        )
    )]
    pub time_unit: Duration,
}
