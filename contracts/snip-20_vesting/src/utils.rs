use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{BlockInfo, StdError, StdResult};
use std::fmt;
use std::ops::{Add, Mul};


#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
/// at the given point in time and after, Expiration will be considered expired
pub enum Expiration {
    /// expires at this block height
    AtHeight(u64),
    /// expires at the time in seconds since 01/01/1970
    AtTime(u64),
    /// never expires
    Never,
}

impl fmt::Display for Expiration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expiration::AtHeight(height) => write!(f, "expiration height: {}", height),
            Expiration::AtTime(time) => write!(f, "expiration time: {}", time),
            Expiration::Never => write!(f, "expiration: never"),
        }
    }
}

/// default is Never
impl Default for Expiration {
    fn default() -> Self {
        Expiration::Never
    }
}

impl Expiration {
    /// Returns bool, true if Expiration has expired
    ///
    /// # Arguments
    ///
    /// * `block` - a reference to the BlockInfo containing the time to compare the Expiration to
    pub fn is_expired(&self, block: &BlockInfo) -> bool {
        match self {
            Expiration::AtHeight(height) => block.height >= *height,
            Expiration::AtTime(time) => block.time >= *time,
            Expiration::Never => false,
        }
    }
}
// pub const HOUR: Duration = Duration::Time(60 * 60);
// pub const DAY: Duration = Duration::Time(24 * 60 * 60);
// pub const WEEK: Duration = Duration::Time(7 * 24 * 60 * 60);

/// Duration is a delta of time. You can add it to a BlockInfo or Expiration to
/// move that further in the future. Note that an height-based Duration and
/// a time-based Expiration cannot be combined
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Duration {
    Height(u64),
    /// Time in seconds
    Time(u64),
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Duration::Height(height) => write!(f, "height: {}", height),
            Duration::Time(time) => write!(f, "time: {}", time),
        }
    }
}

impl Duration {
    /// Create an expiration for Duration after current block
    pub fn after(&self, block: &BlockInfo) -> Expiration {
        match self {
            Duration::Height(h) => Expiration::AtHeight(block.height + h),
            Duration::Time(t) => Expiration::AtTime(block.time + (*t)),
        }
    }

    // creates a number just a little bigger, so we can use it to pass expiration point
    pub fn plus_one(&self) -> Duration {
        match self {
            Duration::Height(h) => Duration::Height(h + 1),
            Duration::Time(t) => Duration::Time(t + 1),
        }
    }
}

impl Add<Duration> for Duration {
    type Output = StdResult<Duration>;

    fn add(self, rhs: Duration) -> StdResult<Duration> {
        match (self, rhs) {
            (Duration::Time(t), Duration::Time(t2)) => Ok(Duration::Time(t + t2)),
            (Duration::Height(h), Duration::Height(h2)) => Ok(Duration::Height(h + h2)),
            _ => Err(StdError::generic_err("Cannot add height and time")),
        }
    }
}

impl Mul<u64> for Duration {
    type Output = Duration;

    fn mul(self, rhs: u64) -> Self::Output {
        match self {
            Duration::Time(t) => Duration::Time(t * rhs),
            Duration::Height(h) => Duration::Height(h * rhs),
        }
    }
}

