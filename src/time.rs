use std::fmt;
use std::str::FromStr;

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

/// Customized version of [`NaiveTime`]
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Serialize, Deserialize)]
pub struct Time(pub NaiveTime);

impl Time {
    /// Format string to use on the inner [`NaiveTime`]
    const FMT: &'static str = "%I:%M %p";
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format(Self::FMT))
    }
}

impl FromStr for Time {
    type Err = chrono::format::ParseError;

    fn from_str(s: &str) -> chrono::format::ParseResult<Self> {
        NaiveTime::parse_from_str(s, Self::FMT).map(Time)
    }
}
