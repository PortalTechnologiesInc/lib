use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

use super::model::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "bindings", derive(uniffi::Object))]
pub struct Calendar {
    /// Optional weekday specification (Mon,Tue..Fri)
    weekdays: Option<Vec<Weekday>>,
    /// Year component (can be * or specific years)
    year: TimeComponent<1970, 2099>,
    /// Month component (can be * or 1-12)
    month: TimeComponent<1, 12>,
    /// Day component (can be * or 1-31)
    day: TimeComponent<1, 31>,
    /// Hour component (can be * or 0-23)
    hour: TimeComponent<0, 23>,
    /// Minute component (can be * or 0-59)
    minute: TimeComponent<0, 59>,
    /// Second component (can be * or 0-59)
    second: TimeComponent<0, 59>,
    /// Timezone (can be None, implies UTC)
    timezone: Option<chrono_tz::Tz>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "bindings", derive(uniffi::Record))]
pub struct CalendarWrapper {
    inner: Arc<Calendar>,
}

impl CalendarWrapper {
    pub fn new(calendar: Calendar) -> Self {
        Self {
            inner: Arc::new(calendar),
        }
    }

    pub fn get_calendar(&self) -> &Calendar {
        &self.inner
    }
}

impl Serialize for CalendarWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for CalendarWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(CalendarWrapper {
            inner: Arc::new(Calendar::deserialize(deserializer)?),
        })
    }
}

// #[cfg(feature = "bindings")]
// uniffi::custom_type!(Calendar, String, {
//     lower: |s| s.to_string(),
//     try_lift: |s| Ok(Calendar::from_str(&s)?),
// });

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeComponent<const MIN: u32, const MAX: u32> {
    /// Matches any value (*)
    Any,
    /// Matches specific values
    Values(Vec<u32>),
    /// Matches a range of values with optional step
    Range {
        start: u32,
        end: u32,
        step: Option<u32>,
    },
}

impl<const MIN: u32, const MAX: u32> TimeComponent<MIN, MAX> {
    pub fn contains(&self, v: u32) -> bool {
        match self {
            TimeComponent::Any => true,
            TimeComponent::Values(values) => values.contains(&v),
            TimeComponent::Range { start, end, step } => {
                let step = step.unwrap_or(1);
                if v < *start || v > *end {
                    false
                } else if (v - *start) % step != 0 {
                    false
                } else {
                    true
                }
            }
        }
    }

    pub fn iter(&self, from: Option<u32>) -> Box<dyn Iterator<Item = u32>> {
        match self {
            TimeComponent::Any => Box::new((from.unwrap_or(MIN)..=MAX).into_iter()),
            TimeComponent::Values(values) => {
                let pos = match values.binary_search(&from.unwrap_or(MIN)) {
                    Ok(i) => i,
                    Err(i) => i,
                };
                Box::new(values.clone().into_iter().skip(pos))
            }
            TimeComponent::Range { start, end, step } => {
                let range = (*start..=*end)
                    .step_by(step.unwrap_or(1) as usize)
                    .filter(move |v| *v >= from.unwrap_or(MIN));
                Box::new(range)
            }
        }
    }
}

// #[cfg(feature = "bindings")]
// macro_rules! create_specialized_time_component {
//     ($name:ident, $min:expr, $max:expr) => {
//         #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//         pub struct $name(pub TimeComponent<$min, $max>);
//         impl Deref for $name {
//             type Target = TimeComponent<$min, $max>;
//
//             fn deref(&self) -> &Self::Target {
//                 &self.0
//             }
//         }
//         impl From<TimeComponent<$min, $max>> for $name {
//             fn from(value: TimeComponent<$min, $max>) -> Self {
//                 $name(value)
//             }
//         }
//         impl core::fmt::Display for $name {
//             fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//                 write!(f, "{}", self.0)
//             }
//         }
//
//         uniffi::custom_type!($name, TimeComponent<0, 2099>, {
//             lower: |s| s.lower(),
//             try_lift: |s| Ok($name(s.try_lift()?)),
//         });
//     };
// }

// #[cfg(feature = "bindings")]
// create_specialized_time_component!(TimeComponentYear, 1970, 2099);
// #[cfg(feature = "bindings")]
// create_specialized_time_component!(TimeComponentMonth, 1, 12);
// #[cfg(feature = "bindings")]
// create_specialized_time_component!(TimeComponentDay, 1, 31);
// #[cfg(feature = "bindings")]
// create_specialized_time_component!(TimeComponentHour, 0, 23);
// #[cfg(feature = "bindings")]
// create_specialized_time_component!(TimeComponentMinute, 0, 59);
// #[cfg(feature = "bindings")]
// create_specialized_time_component!(TimeComponentSecond, 0, 59);

// #[cfg(feature = "bindings")]
// impl<const MIN: u32, const MAX: u32> TimeComponent<MIN, MAX> {
//     fn lower(&self) -> TimeComponent<0, 2099> {
//         match self {
//             TimeComponent::Any => TimeComponent::Any,
//             TimeComponent::Values(values) => TimeComponent::Values(values.clone()),
//             TimeComponent::Range { start, end, step } => TimeComponent::Range { start: *start, end: *end, step: *step },
//         }
//     }
// }
// #[cfg(feature = "bindings")]
// impl TimeComponent<0, 2099> {
//     fn try_lift<const MIN: u32, const MAX: u32>(&self) -> anyhow::Result<TimeComponent<MIN, MAX>> {
//         match self {
//             TimeComponent::Any => Ok(TimeComponent::Any),
//             TimeComponent::Values(values) => {
//                 if values.iter().any(|v| *v < MIN || *v > MAX) {
//                     return Err(anyhow::anyhow!("Value {} is out of range", values[0]));
//                 }
//                 Ok(TimeComponent::Values(values.clone()))
//             },
//             TimeComponent::Range { start, end, step } => {
//                 if *start < MIN || *end > MAX {
//                     return Err(anyhow::anyhow!("Range {}-{} is out of range", start, end));
//                 }
//                 Ok(TimeComponent::Range { start: *start, end: *end, step: *step })
//             }
//         }
//     }
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weekday {
    Mon = 0,
    Tue = 1,
    Wed = 2,
    Thu = 3,
    Fri = 4,
    Sat = 5,
    Sun = 6,
}

#[derive(Debug, Error)]
#[cfg_attr(feature = "bindings", derive(uniffi::Error))]
pub enum CalendarError {
    #[error("Invalid calendar format")]
    InvalidFormat,
    #[error("Invalid weekday: {0}")]
    InvalidWeekday(String),
    #[error("Invalid time component: {0}")]
    InvalidTimeComponent(String),
    #[error("Invalid range: {start} > {end}")]
    InvalidRange { start: u32, end: u32 },
    #[error("Invalid timezone: {0}")]
    InvalidTimezone(String),
}

impl FromStr for Weekday {
    type Err = CalendarError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s_lower = s.to_lowercase();
        match s_lower.as_str() {
            "mon" | "monday" => Ok(Weekday::Mon),
            "tue" | "tuesday" => Ok(Weekday::Tue),
            "wed" | "wednesday" => Ok(Weekday::Wed),
            "thu" | "thursday" => Ok(Weekday::Thu),
            "fri" | "friday" => Ok(Weekday::Fri),
            "sat" | "saturday" => Ok(Weekday::Sat),
            "sun" | "sunday" => Ok(Weekday::Sun),
            _ => Err(CalendarError::InvalidWeekday(s_lower)),
        }
    }
}

impl fmt::Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Weekday::Mon => "Mon",
                Weekday::Tue => "Tue",
                Weekday::Wed => "Wed",
                Weekday::Thu => "Thu",
                Weekday::Fri => "Fri",
                Weekday::Sat => "Sat",
                Weekday::Sun => "Sun",
            }
        )
    }
}

impl<const MIN: u32, const MAX: u32> FromStr for TimeComponent<MIN, MAX> {
    type Err = CalendarError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "*" {
            return Ok(TimeComponent::Any);
        }

        if s.contains("..") {
            let parts: Vec<&str> = s.split("..").collect();
            if parts.len() != 2 {
                return Err(CalendarError::InvalidTimeComponent(s.to_string()));
            }

            let start = parts[0].parse().map_err(|_| {
                CalendarError::InvalidTimeComponent(format!("Invalid start value: {}", parts[0]))
            })?;

            let (end, step) = if parts[1].contains('/') {
                let step_parts: Vec<&str> = parts[1].split('/').collect();
                if step_parts.len() != 2 {
                    return Err(CalendarError::InvalidTimeComponent(s.to_string()));
                }
                let end = step_parts[0].parse().map_err(|_| {
                    CalendarError::InvalidTimeComponent(format!(
                        "Invalid end value: {}",
                        step_parts[0]
                    ))
                })?;
                let step = step_parts[1].parse().map_err(|_| {
                    CalendarError::InvalidTimeComponent(format!(
                        "Invalid step value: {}",
                        step_parts[1]
                    ))
                })?;
                (end, Some(step))
            } else {
                let end = parts[1].parse().map_err(|_| {
                    CalendarError::InvalidTimeComponent(format!("Invalid end value: {}", parts[1]))
                })?;
                (end, None)
            };

            if start > end {
                return Err(CalendarError::InvalidRange { start, end });
            }

            if start < MIN || end > MAX {
                return Err(CalendarError::InvalidTimeComponent(format!(
                    "Range {}-{} is out of range",
                    start, end
                )));
            }

            Ok(TimeComponent::Range { start, end, step })
        } else if s.contains(',') {
            let mut values = s
                .split(',')
                .map(|v| {
                    v.parse::<u32>()
                        .map_err(|_| {
                            CalendarError::InvalidTimeComponent(format!("Invalid value: {}", v))
                        })
                        .and_then(|v| {
                            if v < MIN || v > MAX {
                                Err(CalendarError::InvalidTimeComponent(format!(
                                    "Value {} is out of range",
                                    v
                                )))
                            } else {
                                Ok(v)
                            }
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            values.sort();
            Ok(TimeComponent::Values(values))
        } else {
            let value = s.parse().map_err(|_| {
                CalendarError::InvalidTimeComponent(format!("Invalid value: {}", s))
            })?;

            if value < MIN || value > MAX {
                return Err(CalendarError::InvalidTimeComponent(format!(
                    "Value {} is out of range",
                    value
                )));
            }

            Ok(TimeComponent::Values(vec![value]))
        }
    }
}

impl<const MIN: u32, const MAX: u32> fmt::Display for TimeComponent<MIN, MAX> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeComponent::Any => write!(f, "*"),
            TimeComponent::Values(values) => {
                let s = values
                    .iter()
                    .map(|v| format!("{:02}", v))
                    .collect::<Vec<_>>()
                    .join(",");
                write!(f, "{}", s)
            }
            TimeComponent::Range { start, end, step } => {
                write!(f, "{:02}..{:02}", start, end)?;
                if let Some(step) = step {
                    write!(f, "/{}", step)?;
                }
                Ok(())
            }
        }
    }
}

impl Calendar {
    pub fn minutely(timezone: Option<chrono_tz::Tz>) -> Self {
        Calendar {
            weekdays: None,
            year: TimeComponent::Any.into(),
            month: TimeComponent::Any.into(),
            day: TimeComponent::Any.into(),
            hour: TimeComponent::Any.into(),
            minute: TimeComponent::Any.into(),
            second: TimeComponent::Values(vec![0]).into(),
            timezone,
        }
    }

    pub fn hourly(timezone: Option<chrono_tz::Tz>) -> Self {
        Calendar {
            weekdays: None,
            year: TimeComponent::Any.into(),
            month: TimeComponent::Any.into(),
            day: TimeComponent::Any.into(),
            hour: TimeComponent::Any.into(),
            minute: TimeComponent::Values(vec![0]).into(),
            second: TimeComponent::Values(vec![0]).into(),
            timezone,
        }
    }

    pub fn daily(timezone: Option<chrono_tz::Tz>) -> Self {
        Calendar {
            weekdays: None,
            year: TimeComponent::Any.into(),
            month: TimeComponent::Any.into(),
            day: TimeComponent::Any.into(),
            hour: TimeComponent::Values(vec![0]).into(),
            minute: TimeComponent::Values(vec![0]).into(),
            second: TimeComponent::Values(vec![0]).into(),
            timezone,
        }
    }

    pub fn weekly(timezone: Option<chrono_tz::Tz>) -> Self {
        Calendar {
            weekdays: Some(vec![Weekday::Mon]),
            year: TimeComponent::Any.into(),
            month: TimeComponent::Any.into(),
            day: TimeComponent::Any.into(),
            hour: TimeComponent::Values(vec![0]).into(),
            minute: TimeComponent::Values(vec![0]).into(),
            second: TimeComponent::Values(vec![0]).into(),
            timezone,
        }
    }

    pub fn monthly(timezone: Option<chrono_tz::Tz>) -> Self {
        Calendar {
            weekdays: None,
            year: TimeComponent::Any.into(),
            month: TimeComponent::Any.into(),
            day: TimeComponent::Values(vec![1]).into(),
            hour: TimeComponent::Values(vec![0]).into(),
            minute: TimeComponent::Values(vec![0]).into(),
            second: TimeComponent::Values(vec![0]).into(),
            timezone,
        }
    }

    pub fn yearly(timezone: Option<chrono_tz::Tz>) -> Self {
        Calendar {
            weekdays: None,
            year: TimeComponent::Any.into(),
            month: TimeComponent::Values(vec![1]).into(),
            day: TimeComponent::Values(vec![1]).into(),
            hour: TimeComponent::Values(vec![0]).into(),
            minute: TimeComponent::Values(vec![0]).into(),
            second: TimeComponent::Values(vec![0]).into(),
            timezone,
        }
    }

    pub fn quarterly(timezone: Option<chrono_tz::Tz>) -> Self {
        Calendar {
            weekdays: None,
            year: TimeComponent::Any.into(),
            month: TimeComponent::Values(vec![1, 4, 7, 10]).into(),
            day: TimeComponent::Values(vec![1]).into(),
            hour: TimeComponent::Values(vec![0]).into(),
            minute: TimeComponent::Values(vec![0]).into(),
            second: TimeComponent::Values(vec![0]).into(),
            timezone,
        }
    }

    pub fn semiannually(timezone: Option<chrono_tz::Tz>) -> Self {
        Calendar {
            weekdays: None,
            year: TimeComponent::Any.into(),
            month: TimeComponent::Values(vec![1, 7]).into(),
            day: TimeComponent::Values(vec![1]).into(),
            hour: TimeComponent::Values(vec![0]).into(),
            minute: TimeComponent::Values(vec![0]).into(),
            second: TimeComponent::Values(vec![0]).into(),
            timezone,
        }
    }

    fn get_named_pattern(&self) -> Option<&str> {
        // Check if this matches any of our named patterns
        if self == &Self::minutely(self.timezone) {
            return Some("Every minute");
        }
        if self == &Self::hourly(self.timezone) {
            return Some("Every hour");
        }
        if self == &Self::daily(self.timezone) {
            return Some("Daily");
        }
        if self == &Self::weekly(self.timezone) {
            return Some("Weekly");
        }
        if self == &Self::monthly(self.timezone) {
            return Some("Monthly");
        }
        if self == &Self::yearly(self.timezone) {
            return Some("Yearly");
        }
        if self == &Self::quarterly(self.timezone) {
            return Some("Quarterly");
        }
        if self == &Self::semiannually(self.timezone) {
            return Some("Semi-annually");
        }
        None
    }

    fn get_frequency_text(&self) -> Option<String> {
        // If not following a specific pattern, describe the recurring schedule
        // Based on what components are Any vs specific
        if matches!(self.year, TimeComponent::Any)
            && matches!(self.month, TimeComponent::Any)
            && matches!(self.day, TimeComponent::Any)
            && matches!(self.hour, TimeComponent::Any)
        {
            if let TimeComponent::Range {
                step: Some(step), ..
            } = &self.minute
            {
                return Some(format!("Every {} minutes", step));
            }
        }

        if matches!(self.year, TimeComponent::Any)
            && matches!(self.month, TimeComponent::Any)
            && matches!(self.day, TimeComponent::Any)
        {
            if let TimeComponent::Range {
                step: Some(step), ..
            } = &self.hour
            {
                return Some(format!("Every {} hours", step));
            }
        }

        if matches!(self.year, TimeComponent::Any) && matches!(self.month, TimeComponent::Any) {
            if let TimeComponent::Range {
                step: Some(step), ..
            } = &self.day
            {
                return Some(format!("Every {} days", step));
            }
        }

        if matches!(self.day, TimeComponent::Any)
            && matches!(self.month, TimeComponent::Any)
            && matches!(self.year, TimeComponent::Any)
        {
            return Some("Every day".to_string());
        }

        // Check for specific day of month
        if let TimeComponent::Values(ref days) = self.day {
            if days.len() == 1 {
                return Some(format!(
                    "On the {} day of each month",
                    Self::ordinal(days[0])
                ));
            }
        }

        // For more complex patterns
        Some("On a custom schedule".to_string())
    }

    // Helper function to convert numbers to ordinals (1st, 2nd, 3rd, etc.)
    fn ordinal(n: u32) -> String {
        let suffix = match (n % 10, n % 100) {
            (1, 11) | (2, 12) | (3, 13) => "th",
            (1, _) => "st",
            (2, _) => "nd",
            (3, _) => "rd",
            _ => "th",
        };

        format!("{}{}", n, suffix)
    }

    fn get_time_text(&self) -> Option<String> {
        // For simple time patterns
        if let (
            TimeComponent::Values(hours),
            TimeComponent::Values(mins),
            TimeComponent::Values(secs),
        ) = (&self.hour, &self.minute, &self.second)
        {
            if hours.len() == 1 && mins.len() == 1 && secs.len() == 1 {
                let h = hours[0];
                let m = mins[0];

                // Special names for common times
                if h == 12 && m == 0 {
                    return Some("noon".to_string());
                }
                if h == 0 && m == 0 {
                    return Some("midnight".to_string());
                }

                // Use AM/PM format
                let period = if h >= 12 { "PM" } else { "AM" };
                let h12 = if h == 0 {
                    12
                } else if h > 12 {
                    h - 12
                } else {
                    h
                };

                return Some(format!("{:02}:{:02} {}", h12, m, period));
            }
        }

        None
    }
}

#[cfg_attr(feature = "bindings", uniffi::export)]
impl Calendar {
    pub fn next_occurrence(&self, from: Timestamp) -> Option<Timestamp> {
        let timezone = self.timezone.as_ref().unwrap_or(&chrono_tz::Tz::UTC);

        let from = DateTime::from_timestamp(from.as_u64() as i64, 0)
            .expect("Invalid timestamp")
            .with_timezone(timezone);

        let is_leap_year = |year: u32| year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let days_in_month = |year: u32, month: u32| match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if is_leap_year(year) {
                    29
                } else {
                    28
                }
            }
            _ => unreachable!(),
        };

        self.year
            .iter(Some(from.year() as u32))
            .map(|y| {
                let is_current_year = y == from.year() as u32;
                let from = (is_current_year).then_some(from.month());
                self.month.iter(from).map(move |m| (is_current_year, y, m))
            })
            .flatten()
            .map(|(is_current_year, y, m)| {
                let is_current_month = is_current_year && m == from.month();
                let from = (is_current_month).then_some(from.day());
                let days_in_month = days_in_month(y, m);
                self.day
                    .iter(from)
                    .map(move |d| (is_current_month, y, m, d))
                    .filter(move |(_, _, _, d)| *d <= days_in_month)
            })
            .flatten()
            .filter(|(_, y, m, d)| {
                let weekday = NaiveDate::from_ymd_opt(*y as i32, *m, *d)
                    .expect("Invalid date")
                    .weekday() as u8;
                self.weekdays
                    .as_ref()
                    .map(|l| l.iter().any(|d| *d as u8 == weekday))
                    .unwrap_or(true)
            })
            .map(|(is_current_month, y, m, d)| {
                let is_current_day = is_current_month && d == from.day();
                let from = (is_current_day).then_some(from.hour());
                self.hour
                    .iter(from)
                    .map(move |h| (is_current_day, y, m, d, h))
            })
            .flatten()
            .map(|(is_current_day, y, m, d, h)| {
                let is_current_hour = is_current_day && h == from.hour();
                let from = (is_current_hour).then_some(from.minute());
                self.minute
                    .iter(from)
                    .map(move |mi| (is_current_hour, y, m, d, h, mi))
            })
            .flatten()
            .map(|(is_current_hour, y, m, d, h, mi)| {
                let is_current_minute = is_current_hour && mi == from.minute();
                let from = (is_current_minute).then_some(from.second());
                self.second
                    .iter(from)
                    .map(move |s| (is_current_minute, y, m, d, h, mi, s))
            })
            .flatten()
            .filter_map(|(_, y, m, d, h, mi, s)| {
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(y as i32, m as u32, d as u32).expect("Invalid date"),
                    NaiveTime::from_hms_opt(h as u32, mi as u32, s as u32).expect("Invalid time"),
                )
                .and_local_timezone(*timezone)
                .earliest()
            })
            .map(|dt| Timestamp::new(dt.timestamp() as u64))
            .next()
    }

    /// Convert a calendar to a human-readable description
    ///
    /// * `show_timezone` - Whether to include the timezone in the description (defaults to true)
    pub fn to_human_readable(&self, show_timezone: bool) -> String {
        // Special case for common patterns
        if let Some(keyword) = self.get_named_pattern() {
            return keyword.to_string();
        }

        let mut parts = Vec::new();

        // Handle frequency: daily, weekly, monthly, etc.
        if let Some(frequency) = self.get_frequency_text() {
            parts.push(frequency);
        }

        // Specific days
        if let Some(ref weekdays) = self.weekdays {
            if weekdays.len() == 1 {
                parts.push(format!("on {}", weekdays[0]));
            } else {
                let days = weekdays
                    .iter()
                    .map(|d| d.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                parts.push(format!("on {}", days));
            }
        }

        // Specific time
        if let Some(time_text) = self.get_time_text() {
            parts.push(format!("at {}", time_text));
        }

        // Add timezone if present and requested
        if show_timezone {
            if let Some(tz) = &self.timezone {
                parts.push(format!("({})", tz));
            }
        }

        parts.join(" ")
    }

    pub fn to_calendar_string(&self) -> String {
        self.to_string()
    }
}

impl FromStr for Calendar {
    type Err = CalendarError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Handle special keywords
        match s.to_lowercase().as_str() {
            "minutely" => return Ok(Calendar::minutely(None)),
            "hourly" => return Ok(Calendar::hourly(None)),
            "daily" => return Ok(Calendar::daily(None)),
            "monthly" => return Ok(Calendar::monthly(None)),
            "weekly" => return Ok(Calendar::weekly(None)),
            "yearly" | "annually" => return Ok(Calendar::yearly(None)),
            "quarterly" => return Ok(Calendar::quarterly(None)),
            "semiannually" => return Ok(Calendar::semiannually(None)),
            _ => {}
        }

        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.is_empty() {
            return Err(CalendarError::InvalidFormat);
        }

        let mut weekdays = None;
        let mut date_parts = None;
        let mut time_parts = None;
        let mut timezone = None;

        for (i, part) in parts.into_iter().enumerate() {
            if part.contains(':') {
                // Time part
                time_parts = Some(part.split(':').collect::<Vec<_>>());
            } else if part.contains('-') || part == "*" {
                if time_parts.is_some() {
                    return Err(CalendarError::InvalidFormat);
                }

                // Date part
                date_parts = Some(part.split('-').collect::<Vec<_>>());
            // } else if part.contains('/') || part.contains('.') {
            //     // Explicitly reject parts with slashes or periods before trying weekday parsing
            //     return Err(CalendarError::InvalidFormat);
            } else if i == 0 {
                // Try parsing as weekday part (single, list, or range)
                let weekday_list = if part.contains("..") {
                    // Parse range
                    let range: Vec<&str> = part.split("..").collect();
                    if range.len() != 2 {
                        return Err(CalendarError::InvalidFormat);
                    }
                    let start = Weekday::from_str(range[0])?;
                    let end = Weekday::from_str(range[1])?;
                    (start as u8..=end as u8)
                        .map(|d| unsafe { std::mem::transmute(d) })
                        .collect()
                } else {
                    // Parse list or single
                    part.split(',')
                        .map(Weekday::from_str)
                        .collect::<Result<Vec<_>, _>>()?
                };
                weekdays = Some(weekday_list);
            } else if time_parts.is_some() {
                // Could be the timezone, but it only comes after the time
                timezone = Some(
                    part.parse::<chrono_tz::Tz>()
                        .map_err(|_| CalendarError::InvalidTimezone(part.to_string()))?,
                );
            } else {
                return Err(CalendarError::InvalidFormat);
            }
        }

        // Parse date components
        let (year, month, day) = match date_parts {
            Some(parts) => {
                let year = parts.get(0).unwrap_or(&"*").parse()?;
                let month = parts.get(1).unwrap_or(&"*").parse()?;
                let day = parts.get(2).unwrap_or(&"*").parse()?;
                (year, month, day)
            }
            None => (TimeComponent::Any, TimeComponent::Any, TimeComponent::Any),
        };

        // Parse time components
        let (hour, minute, second) = match time_parts {
            Some(parts) => {
                let hour = parts.get(0).unwrap_or(&"*").parse()?;
                let minute = parts.get(1).unwrap_or(&"*").parse()?;
                let second = parts.get(2).unwrap_or(&"0").parse()?;

                (hour, minute, second)
            }
            None => (
                TimeComponent::Any,
                TimeComponent::Any,
                TimeComponent::Values(vec![0]),
            ),
        };

        Ok(Calendar {
            weekdays,
            year: year.into(),
            month: month.into(),
            day: day.into(),
            hour: hour.into(),
            minute: minute.into(),
            second: second.into(),
            timezone,
        })
    }
}

impl fmt::Display for Calendar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref weekdays) = self.weekdays {
            let weekday_str = weekdays
                .iter()
                .map(|w| w.to_string())
                .collect::<Vec<_>>()
                .join(",");
            write!(f, "{} ", weekday_str)?;
        }

        // Always include date part
        write!(f, "{}-{}-{} ", self.year, self.month, self.day)?;

        // Time part
        write!(f, "{}:{}:{}", self.hour, self.minute, self.second)
    }
}

impl Serialize for Calendar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Calendar {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_weekdays() {
        let cal: Calendar = "Mon,Wed,Fri *-*-* 00:00:00".parse().unwrap();
        assert_eq!(
            cal.weekdays,
            Some(vec![Weekday::Mon, Weekday::Wed, Weekday::Fri])
        );
    }

    #[test]
    fn test_parse_weekday_range() {
        let cal: Calendar = "Mon..Fri *-*-* 00:00:00".parse().unwrap();
        assert_eq!(
            cal.weekdays,
            Some(vec![
                Weekday::Mon,
                Weekday::Tue,
                Weekday::Wed,
                Weekday::Thu,
                Weekday::Fri
            ])
        );
    }

    #[test]
    fn test_parse_time_component() {
        assert_eq!(
            TimeComponent::<0, 23>::from_str("*").unwrap(),
            TimeComponent::Any
        );
        assert_eq!(
            TimeComponent::<0, 23>::from_str("1,2,3").unwrap(),
            TimeComponent::Values(vec![1, 2, 3])
        );
        assert_eq!(
            TimeComponent::<0, 23>::from_str("1..5").unwrap(),
            TimeComponent::Range {
                start: 1,
                end: 5,
                step: None
            }
        );
        assert_eq!(
            TimeComponent::<0, 59>::from_str("1..10/2").unwrap(),
            TimeComponent::Range {
                start: 1,
                end: 10,
                step: Some(2)
            }
        );
    }

    #[test]
    fn test_parse_special_keywords() {
        assert_eq!(
            Calendar::from_str("minutely").unwrap(),
            Calendar::minutely(None)
        );
        assert_eq!(
            Calendar::from_str("hourly").unwrap(),
            Calendar::hourly(None)
        );
        assert_eq!(Calendar::from_str("daily").unwrap(), Calendar::daily(None));
        assert_eq!(
            Calendar::from_str("weekly").unwrap(),
            Calendar::weekly(None)
        );
        assert_eq!(
            Calendar::from_str("monthly").unwrap(),
            Calendar::monthly(None)
        );
        assert_eq!(
            Calendar::from_str("yearly").unwrap(),
            Calendar::yearly(None)
        );
        assert_eq!(
            Calendar::from_str("quarterly").unwrap(),
            Calendar::quarterly(None)
        );
        assert_eq!(
            Calendar::from_str("semiannually").unwrap(),
            Calendar::semiannually(None)
        );
    }

    #[test]
    fn test_format_calendar() {
        let cal = Calendar {
            weekdays: Some(vec![Weekday::Mon, Weekday::Wed, Weekday::Fri]),
            year: TimeComponent::Any.into(),
            month: TimeComponent::Any.into(),
            day: TimeComponent::Any.into(),
            hour: TimeComponent::Values(vec![0]).into(),
            minute: TimeComponent::Values(vec![0]).into(),
            second: TimeComponent::Values(vec![0]).into(),
            timezone: None,
        };
        assert_eq!(cal.to_string(), "Mon,Wed,Fri *-*-* 0:0:0");
    }

    #[test]
    fn test_invalid_formats() {
        // Invalid weekday
        assert!(matches!(
            Calendar::from_str("Foo *-*-* 00:00:00"),
            Err(CalendarError::InvalidWeekday(s)) if s == "foo"
        ));

        // Invalid time component
        assert!(matches!(
            Calendar::from_str("Mon *-*-* 25:00:00"),
            Err(CalendarError::InvalidTimeComponent(_))
        ));

        // Invalid range (start > end)
        assert!(matches!(
            Calendar::from_str("Mon *-*-* 10..5:00:00"),
            Err(CalendarError::InvalidFormat)
        ));

        // Empty string
        assert!(matches!(
            Calendar::from_str(""),
            Err(CalendarError::InvalidFormat)
        ));

        // Invalid date format
        assert!(matches!(
            Calendar::from_str("2023/12/25"),
            Err(CalendarError::InvalidFormat)
        ));

        // Invalid time format
        assert!(matches!(
            Calendar::from_str("*-*-* 10.30.00"),
            Err(CalendarError::InvalidFormat)
        ));

        // Invalid weekday range
        assert!(matches!(
            Calendar::from_str("Mon..Invalid *-*-* 00:00:00"),
            Err(CalendarError::InvalidFormat),
        ));

        // Invalid step value
        assert!(matches!(
            Calendar::from_str("*-*-* 1..10/invalid:00:00"),
            Err(CalendarError::InvalidFormat)
        ));
    }

    #[test]
    fn test_next_occurrence() {
        let s = "Mon,Wed 2020..2030/2-03..06/3-01,05,06 08..09/3:00,30:00 Europe/Rome";
        dbg!(&s);
        let cal: Calendar = s.parse().unwrap();
        let last_occurrence = cal.next_occurrence(Timestamp::new(1744993853));
        assert_eq!(last_occurrence, Some(Timestamp::new(1780293600)));
        dbg!(
            chrono::DateTime::from_timestamp(last_occurrence.unwrap().as_u64() as i64, 0)
                .unwrap()
                .with_timezone(&chrono_tz::Europe::Rome)
        );

        let cal = dbg!("Thu *-02-29").parse::<Calendar>().unwrap();
        let last_occurrence = cal.next_occurrence(Timestamp::new(1744993853));
        assert_eq!(last_occurrence, Some(Timestamp::new(2592777600)));
        dbg!(
            chrono::DateTime::from_timestamp(last_occurrence.unwrap().as_u64() as i64, 0)
                .unwrap()
                .with_timezone(&chrono_tz::Europe::Rome)
        );

        let x = 1745499045;
        let cal = dbg!("*-*-* *:*:00").parse::<Calendar>().unwrap();
        let next_occurrence = cal.next_occurrence(Timestamp::new(x));
        assert!(next_occurrence.unwrap() > Timestamp::new(x));

        let x = 1745499045;
        let cal = dbg!("*-*-* *:0..59/10:0").parse::<Calendar>().unwrap();
        dbg!(cal.to_human_readable(false));
        let next_occurrence = cal.next_occurrence(Timestamp::new(x));
        assert!(next_occurrence.unwrap() > Timestamp::new(x));
        dbg!(
            chrono::DateTime::from_timestamp(x as i64, 0)
                .unwrap()
                .with_timezone(&chrono_tz::Europe::Rome)
        );
        dbg!(
            chrono::DateTime::from_timestamp(next_occurrence.unwrap().as_u64() as i64, 0)
                .unwrap()
                .with_timezone(&chrono_tz::Europe::Rome)
        );
    }

    #[test]
    fn test_serialize() {
        let cal = Calendar::minutely(None);
        let s = serde_json::to_string(&cal).unwrap();
        dbg!(&s);
    }

    #[test]
    fn test_human_readable_formats() {
        // Test standard named patterns
        assert_eq!(
            Calendar::minutely(None).to_human_readable(true),
            "Every minute"
        );
        assert_eq!(Calendar::hourly(None).to_human_readable(true), "Every hour");
        assert_eq!(Calendar::daily(None).to_human_readable(true), "Daily");
        assert_eq!(Calendar::weekly(None).to_human_readable(true), "Weekly");
        assert_eq!(Calendar::monthly(None).to_human_readable(true), "Monthly");
        assert_eq!(Calendar::yearly(None).to_human_readable(true), "Yearly");
        assert_eq!(
            Calendar::quarterly(None).to_human_readable(true),
            "Quarterly"
        );
        assert_eq!(
            Calendar::semiannually(None).to_human_readable(true),
            "Semi-annually"
        );

        // Test with timezone
        let tz = Some(chrono_tz::US::Eastern);
        assert_eq!(Calendar::daily(tz.clone()).to_human_readable(true), "Daily");

        // Test timezone parameter
        let cal_with_tz = Calendar {
            weekdays: None,
            year: TimeComponent::Any,
            month: TimeComponent::Any,
            day: TimeComponent::Values(vec![15]),
            hour: TimeComponent::Values(vec![14]),
            minute: TimeComponent::Values(vec![30]),
            second: TimeComponent::Values(vec![0]),
            timezone: tz,
        };
        assert_eq!(
            cal_with_tz.to_human_readable(true),
            "On the 15th day of each month at 02:30 PM (US/Eastern)"
        );
        assert_eq!(
            cal_with_tz.to_human_readable(false),
            "On the 15th day of each month at 02:30 PM"
        );

        // Test ordinal day formats
        let cal_1st = Calendar {
            weekdays: None,
            year: TimeComponent::Any,
            month: TimeComponent::Any,
            day: TimeComponent::Values(vec![1]),
            hour: TimeComponent::Values(vec![9]),
            minute: TimeComponent::Values(vec![0]),
            second: TimeComponent::Values(vec![0]),
            timezone: None,
        };
        assert_eq!(
            cal_1st.to_human_readable(true),
            "On the 1st day of each month at 09:00 AM"
        );

        let cal_2nd = Calendar {
            weekdays: None,
            year: TimeComponent::Any,
            month: TimeComponent::Any,
            day: TimeComponent::Values(vec![2]),
            hour: TimeComponent::Values(vec![14]),
            minute: TimeComponent::Values(vec![30]),
            second: TimeComponent::Values(vec![0]),
            timezone: None,
        };
        assert_eq!(
            cal_2nd.to_human_readable(true),
            "On the 2nd day of each month at 02:30 PM"
        );

        let cal_3rd = Calendar {
            weekdays: None,
            year: TimeComponent::Any,
            month: TimeComponent::Any,
            day: TimeComponent::Values(vec![3]),
            hour: TimeComponent::Values(vec![0]),
            minute: TimeComponent::Values(vec![0]),
            second: TimeComponent::Values(vec![0]),
            timezone: None,
        };
        assert_eq!(
            cal_3rd.to_human_readable(true),
            "On the 3rd day of each month at midnight"
        );

        let cal_4th = Calendar {
            weekdays: None,
            year: TimeComponent::Any,
            month: TimeComponent::Any,
            day: TimeComponent::Values(vec![4]),
            hour: TimeComponent::Values(vec![12]),
            minute: TimeComponent::Values(vec![0]),
            second: TimeComponent::Values(vec![0]),
            timezone: None,
        };
        assert_eq!(
            cal_4th.to_human_readable(true),
            "On the 4th day of each month at noon"
        );

        let cal_11th = Calendar {
            weekdays: None,
            year: TimeComponent::Any,
            month: TimeComponent::Any,
            day: TimeComponent::Values(vec![11]),
            hour: TimeComponent::Values(vec![9]),
            minute: TimeComponent::Values(vec![15]),
            second: TimeComponent::Values(vec![0]),
            timezone: None,
        };
        assert_eq!(
            cal_11th.to_human_readable(true),
            "On the 11th day of each month at 09:15 AM"
        );

        let cal_21st = Calendar {
            weekdays: None,
            year: TimeComponent::Any,
            month: TimeComponent::Any,
            day: TimeComponent::Values(vec![21]),
            hour: TimeComponent::Values(vec![9]),
            minute: TimeComponent::Values(vec![15]),
            second: TimeComponent::Values(vec![0]),
            timezone: None,
        };
        assert_eq!(
            cal_21st.to_human_readable(true),
            "On the 21st day of each month at 09:15 AM"
        );

        // Test weekday formatting
        let cal_weekday = Calendar {
            weekdays: Some(vec![Weekday::Mon, Weekday::Wed, Weekday::Fri]),
            year: TimeComponent::Any,
            month: TimeComponent::Any,
            day: TimeComponent::Any,
            hour: TimeComponent::Values(vec![17]),
            minute: TimeComponent::Values(vec![0]),
            second: TimeComponent::Values(vec![0]),
            timezone: None,
        };
        assert_eq!(
            cal_weekday.to_human_readable(true),
            "Every day on Mon, Wed, Fri at 05:00 PM"
        );

        // Test with timezone
        let cal_tz = Calendar {
            weekdays: None,
            year: TimeComponent::Any,
            month: TimeComponent::Any,
            day: TimeComponent::Values(vec![15]),
            hour: TimeComponent::Values(vec![14]),
            minute: TimeComponent::Values(vec![30]),
            second: TimeComponent::Values(vec![0]),
            timezone: Some(chrono_tz::Europe::London),
        };
        assert_eq!(
            cal_tz.to_human_readable(true),
            "On the 15th day of each month at 02:30 PM (Europe/London)"
        );

        let cal_every_10 = Calendar {
            weekdays: None,
            year: TimeComponent::Any,
            month: TimeComponent::Any,
            day: TimeComponent::Any,
            hour: TimeComponent::Any,
            minute: TimeComponent::Range {
                start: 0,
                end: 59,
                step: Some(10),
            },
            second: TimeComponent::Values(vec![0]),
            timezone: None,
        };
        assert_eq!(cal_every_10.to_human_readable(true), "Every 10 minutes");

        let cal_every_6h = Calendar {
            weekdays: None,
            year: TimeComponent::Any,
            month: TimeComponent::Any,
            day: TimeComponent::Any,
            hour: TimeComponent::Range {
                start: 0,
                end: 23,
                step: Some(6),
            },
            minute: TimeComponent::Values(vec![0]),
            second: TimeComponent::Values(vec![0]),
            timezone: None,
        };
        assert_eq!(cal_every_6h.to_human_readable(true), "Every 6 hours");

        let cal_every_3d = Calendar {
            weekdays: None,
            year: TimeComponent::Any,
            month: TimeComponent::Any,
            day: TimeComponent::Range {
                start: 1,
                end: 31,
                step: Some(3),
            },
            hour: TimeComponent::Values(vec![0]),
            minute: TimeComponent::Values(vec![0]),
            second: TimeComponent::Values(vec![0]),
            timezone: None,
        };
        assert_eq!(
            cal_every_3d.to_human_readable(true),
            "Every 3 days at midnight"
        );
    }
}
