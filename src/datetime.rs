//! Native date/time value model for the Rust port.
//!
//! This module owns deterministic date/time invariants only. String parsing,
//! localized formatting, current-time functions, and timezone policy are owned
//! by later Epic 10 tasks.
//!
//! Upstream oracle:
//! - `../libqalculate/libqalculate/QalculateDateTime.h`
//! - `../libqalculate/libqalculate/QalculateDateTime.cc`

use crate::number::{Number, NumberValue, Rational};
use rug::{Integer, Rational as RugRational};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

const SECONDS_PER_DAY: i64 = 86_400;
const GREGORIAN_FIXED_UNIX_EPOCH: f64 = 719_163.0;
const J2000: f64 = 730_120.5;
const MEAN_SYNODIC_MONTH: f64 = 29.530_588_861;
const LS_FIRST_YEAR: i64 = 1972;
const LS_LAST_YEAR: i64 = 2016;

#[rustfmt::skip]
const HAS_LEAP_SECOND: [bool; 90] = [
    true, true, // 1972
    false, true,
    false, true,
    false, true,
    false, true,
    false, true,
    false, true,
    false, true,
    false, false, // 1980
    true, false,
    true, false,
    true, false,
    false, false,
    true, false,
    false, false,
    false, true,
    false, false,
    false, true,
    false, true, // 1990
    false, false,
    true, false,
    true, false,
    true, false,
    false, true,
    false, false,
    true, false,
    false, true,
    false, false,
    false, false, // 2000
    false, false,
    false, false,
    false, false,
    false, false,
    false, true,
    false, false,
    false, false,
    false, true,
    false, false,
    false, false, // 2010
    false, false,
    true, false,
    false, false,
    false, false,
    true, false,
    false, true, // 2016
];

/// Error returned when constructing or transforming a date/time value fails.
#[derive(Debug, Clone, PartialEq)]
pub enum DateTimeError {
    /// Month was outside the inclusive `1..=12` range.
    InvalidMonth {
        /// Rejected month value.
        month: i64,
    },
    /// Day was outside the valid range for the given year and month.
    InvalidDay {
        /// Year used for validation.
        year: i64,
        /// Month used for validation.
        month: i64,
        /// Rejected day value.
        day: i64,
    },
    /// Hour was outside the inclusive `0..=23` range.
    InvalidHour {
        /// Rejected hour value.
        hour: i64,
    },
    /// Minute was outside the inclusive `0..=59` range.
    InvalidMinute {
        /// Rejected minute value.
        minute: i64,
    },
    /// Seconds were not finite real seconds in the supported range.
    InvalidSecond {
        /// Rejected second value.
        second: Number,
    },
    /// Duration was not a finite exact real value supported by the model.
    InvalidDuration {
        /// Rejected duration value.
        duration: Number,
    },
    /// Timestamp or date arithmetic result exceeded the supported range.
    OutOfRange,
}

impl std::fmt::Display for DateTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMonth { month } => write!(f, "invalid month {month}"),
            Self::InvalidDay { year, month, day } => {
                write!(f, "invalid day {day} for {year:04}-{month:02}")
            }
            Self::InvalidHour { hour } => write!(f, "invalid hour {hour}"),
            Self::InvalidMinute { minute } => write!(f, "invalid minute {minute}"),
            Self::InvalidSecond { second } => write!(f, "invalid second {second}"),
            Self::InvalidDuration { duration } => {
                write!(f, "invalid date/time duration: {duration}")
            }
            Self::OutOfRange => write!(f, "date/time value is out of range"),
        }
    }
}

impl std::error::Error for DateTimeError {}

/// Error returned when parsing a date/time literal fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateTimeParseError {
    message: String,
}

impl DateTimeParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DateTimeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DateTimeParseError {}

/// A parsed date/time plus the optional source timezone offset in minutes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDateTime {
    value: DateTime,
    offset_minutes: Option<i32>,
}

impl ParsedDateTime {
    /// Returns the validated date/time value.
    pub fn value(&self) -> &DateTime {
        &self.value
    }

    /// Returns the parsed timezone offset in minutes, when present.
    pub fn offset_minutes(&self) -> Option<i32> {
        self.offset_minutes
    }
}

/// Validated Gregorian date/time value compatible with upstream `QalculateDateTime`.
#[derive(Debug, Clone)]
pub struct DateTime {
    year: i64,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: Number,
    time_is_set: bool,
}

impl DateTime {
    /// Creates a date without an explicit time component.
    pub fn from_ymd(year: i64, month: i64, day: i64) -> Result<Self, DateTimeError> {
        validate_date(year, month, day)?;

        Ok(Self {
            year,
            month: month as u8,
            day: day as u8,
            hour: 0,
            minute: 0,
            second: Number::new(),
            time_is_set: false,
        })
    }

    /// Creates a date/time with explicit hour, minute, and exact seconds.
    pub fn from_ymd_hms(
        year: i64,
        month: i64,
        day: i64,
        hour: i64,
        minute: i64,
        second: Number,
    ) -> Result<Self, DateTimeError> {
        validate_date(year, month, day)?;
        validate_time(year, month, day, hour, minute, &second)?;

        Ok(Self {
            year,
            month: month as u8,
            day: day as u8,
            hour: hour as u8,
            minute: minute as u8,
            second,
            time_is_set: true,
        })
    }

    /// Returns the UTC Unix epoch as `1970-01-01T00:00:00Z`.
    pub fn epoch_utc() -> Self {
        Self {
            year: 1970,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: Number::new(),
            time_is_set: true,
        }
    }

    /// Creates a UTC date/time from an exact Unix timestamp in seconds.
    pub fn from_timestamp_utc(timestamp: &Number) -> Result<Self, DateTimeError> {
        let rational =
            exact_real_rational(timestamp).ok_or_else(|| DateTimeError::InvalidSecond {
                second: timestamp.clone(),
            })?;

        let mut days_rational = rational.clone();
        days_rational /= SECONDS_PER_DAY;
        let day_count = floor_rational_to_integer(&days_rational);
        let mut seconds_of_day = rational;
        seconds_of_day -= RugRational::from(day_count.clone() * SECONDS_PER_DAY);

        let day_i64 = day_count.to_i64().ok_or(DateTimeError::OutOfRange)?;
        let (year, month, day) = civil_from_days(day_i64);

        let whole_seconds = floor_rational_to_integer(&seconds_of_day);
        let whole_seconds_i64 = whole_seconds.to_i64().ok_or(DateTimeError::OutOfRange)?;
        let hour = whole_seconds_i64 / 3_600;
        let minute = (whole_seconds_i64 % 3_600) / 60;
        let minute_start = hour * 3_600 + minute * 60;
        seconds_of_day -= RugRational::from(minute_start);
        let second = number_from_rational(seconds_of_day);

        Self::from_ymd_hms(year, month, day, hour, minute, second)
    }

    /// Returns the Gregorian year.
    pub fn year(&self) -> i64 {
        self.year
    }

    /// Returns the Gregorian month in the inclusive `1..=12` range.
    pub fn month(&self) -> i64 {
        i64::from(self.month)
    }

    /// Returns the day of month.
    pub fn day(&self) -> i64 {
        i64::from(self.day)
    }

    /// Returns the hour in the inclusive `0..=23` range.
    pub fn hour(&self) -> i64 {
        i64::from(self.hour)
    }

    /// Returns the minute in the inclusive `0..=59` range.
    pub fn minute(&self) -> i64 {
        i64::from(self.minute)
    }

    /// Returns the exact second value.
    pub fn second(&self) -> &Number {
        &self.second
    }

    /// Returns whether an explicit time component is set.
    pub fn time_is_set(&self) -> bool {
        self.time_is_set
    }

    /// Returns a copy with integer or fractional days added.
    pub fn add_days(&self, days: &Number) -> Result<Self, DateTimeError> {
        if days.is_integer() {
            let day_delta = days
                .to_i64()
                .ok_or_else(|| DateTimeError::InvalidDuration {
                    duration: days.clone(),
                })?;
            let absolute_days = days_from_civil(self.year, self.month(), self.day())
                .checked_add(day_delta)
                .ok_or(DateTimeError::OutOfRange)?;
            let (year, month, day) = civil_from_days(absolute_days);
            return Self::from_ymd_hms(
                year,
                month,
                day,
                self.hour(),
                self.minute(),
                self.second.clone(),
            )
            .map(|mut value| {
                value.time_is_set = self.time_is_set;
                value
            });
        }

        let seconds = days.mul(&Number::from_i64(SECONDS_PER_DAY));
        self.add_seconds(&seconds)
    }

    /// Returns a copy with exact integer months added.
    pub fn add_months(&self, months: &Number) -> Result<Self, DateTimeError> {
        let (months, fraction) = split_truncated_fraction(months)?;
        let value = self.add_whole_months(months)?;
        if fraction.is_zero() {
            return Ok(value);
        }

        let days_in_current_month =
            Number::from_i64(i64::from(days_per_month(value.month(), value.year)));
        let mut days = fraction;

        if days.is_negative() {
            days = days.negate().mul(&days_in_current_month);
            if days >= Number::from_i64(value.day() - 1) {
                days = days.div(&days_in_current_month);
                let mut day_fraction = Number::from_i64(value.day() - 1)
                    .add(&value.seconds_fraction_of_day())
                    .div(&days_in_current_month);
                days = days.sub(&day_fraction);
                let previous_month = if value.month == 1 {
                    12
                } else {
                    value.month - 1
                };
                days = days.mul(&Number::from_i64(i64::from(days_per_month(
                    i64::from(previous_month),
                    value.year,
                ))));
                day_fraction = day_fraction.mul(&days_in_current_month);
                days = days.add(&day_fraction);
            }
            days = days.negate();
        } else {
            days = days.mul(&days_in_current_month);
            if days
                >= Number::from_i64(
                    i64::from(days_per_month(value.month(), value.year)) - value.day(),
                )
            {
                days = days.div(&days_in_current_month);
                let mut day_fraction = Number::from_i64(
                    i64::from(days_per_month(value.month(), value.year)) - value.day(),
                )
                .sub(&value.seconds_fraction_of_day())
                .div(&days_in_current_month);
                days = days.sub(&day_fraction);
                let next_month = if value.month == 12 {
                    1
                } else {
                    value.month + 1
                };
                days = days.mul(&Number::from_i64(i64::from(days_per_month(
                    i64::from(next_month),
                    value.year,
                ))));
                day_fraction = day_fraction.mul(&days_in_current_month);
                days = days.add(&day_fraction);
            }
        }

        value.add_days(&days)
    }

    fn add_whole_months(&self, months: i64) -> Result<Self, DateTimeError> {
        let mut year = self.year;
        let mut month = self.month() + months % 12;
        year = year
            .checked_add(months / 12)
            .ok_or(DateTimeError::OutOfRange)?;

        if month > 12 {
            month -= 12;
            year = year.checked_add(1).ok_or(DateTimeError::OutOfRange)?;
        } else if month < 1 {
            month += 12;
            year = year.checked_sub(1).ok_or(DateTimeError::OutOfRange)?;
        }

        let mut day = self.day();
        if day > i64::from(days_per_month(month, year)) {
            day -= i64::from(days_per_month(month, year));
            month += 1;
            if month > 12 {
                month = 1;
                year = year.checked_add(1).ok_or(DateTimeError::OutOfRange)?;
            }
        }

        Self::from_ymd_hms(
            year,
            month,
            day,
            self.hour(),
            self.minute(),
            self.second.clone(),
        )
        .map(|mut value| {
            value.time_is_set = self.time_is_set;
            value
        })
    }

    /// Returns a copy with exact integer years added.
    pub fn add_years(&self, years: &Number) -> Result<Self, DateTimeError> {
        let (years, fraction) = split_truncated_fraction(years)?;
        let value = self.add_whole_years(years)?;
        if fraction.is_zero() {
            return Ok(value);
        }

        let mut days = fraction;
        let days_in_current_year = Number::from_i64(i64::from(days_per_year(value.year)));
        let year_day = value.year_day();
        if days.is_negative() {
            days = days.negate().mul(&days_in_current_year);
            if days >= Number::from_i64(year_day - 1) {
                days = days.div(&days_in_current_year);
                let mut day_fraction = Number::from_i64(year_day - 1)
                    .add(&value.seconds_fraction_of_day())
                    .div(&days_in_current_year);
                days = days.sub(&day_fraction);
                days = days.mul(&Number::from_i64(i64::from(days_per_year(value.year - 1))));
                day_fraction = day_fraction.mul(&days_in_current_year);
                days = days.add(&day_fraction);
            }
            days = days.negate();
        } else {
            days = days.mul(&days_in_current_year);
            if days >= Number::from_i64(i64::from(days_per_year(value.year)) - year_day) {
                days = days.div(&days_in_current_year);
                // Match upstream's year-position rescaling when the fractional
                // offset crosses into the next year.
                let mut day_fraction = Number::from_i64(year_day - 1)
                    .sub(&value.seconds_fraction_of_day())
                    .div(&days_in_current_year);
                days = days.sub(&day_fraction);
                days = days.mul(&Number::from_i64(i64::from(days_per_year(value.year + 1))));
                day_fraction = day_fraction.mul(&days_in_current_year);
                days = days.add(&day_fraction);
            }
        }

        value.add_days(&days)
    }

    fn add_whole_years(&self, years: i64) -> Result<Self, DateTimeError> {
        let mut year = self
            .year
            .checked_add(years)
            .ok_or(DateTimeError::OutOfRange)?;
        let mut month = self.month();
        let mut day = self.day();

        if day > i64::from(days_per_month(month, year)) {
            day -= i64::from(days_per_month(month, year));
            month += 1;
            if month > 12 {
                month = 1;
                year = year.checked_add(1).ok_or(DateTimeError::OutOfRange)?;
            }
        }

        Self::from_ymd_hms(
            year,
            month,
            day,
            self.hour(),
            self.minute(),
            self.second.clone(),
        )
        .map(|mut value| {
            value.time_is_set = self.time_is_set;
            value
        })
    }

    /// Returns a copy with exact seconds added in UTC.
    pub fn add_seconds(&self, seconds: &Number) -> Result<Self, DateTimeError> {
        let timestamp = self.timestamp_utc().add(seconds);
        Self::from_timestamp_utc(&timestamp)
    }

    /// Returns the UTC Unix timestamp in exact seconds.
    pub fn timestamp_utc(&self) -> Number {
        let days = days_from_civil(self.year, self.month(), self.day());
        let day_seconds = Integer::from(days) * SECONDS_PER_DAY;
        let clock_seconds = i64::from(self.hour) * 3_600 + i64::from(self.minute) * 60;
        let mut rational = RugRational::from(day_seconds + clock_seconds);
        rational += exact_real_rational(&self.second).expect("validated date/time seconds");
        number_from_rational(rational)
    }

    /// Returns exact seconds from this date/time to `other`.
    pub fn seconds_to(&self, other: &Self) -> Number {
        other.timestamp_utc().sub(&self.timestamp_utc())
    }

    /// Returns exact days from this date/time to `other`.
    pub fn days_to(&self, other: &Self) -> Number {
        self.seconds_to(other)
            .div(&Number::from_i64(SECONDS_PER_DAY))
    }

    fn year_day(&self) -> i64 {
        (1..self.month())
            .map(|month| i64::from(days_per_month(month, self.year)))
            .sum::<i64>()
            + self.day()
    }

    fn seconds_fraction_of_day(&self) -> Number {
        Number::from_i64(i64::from(self.hour) * 3_600 + i64::from(self.minute) * 60)
            .add(&self.second)
            .div(&Number::from_i64(SECONDS_PER_DAY))
    }

    pub(crate) fn source_string(&self) -> String {
        if !self.time_is_set {
            return format!("{:04}-{:02}-{:02}", self.year, self.month, self.day);
        }

        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{}",
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            format_second_component(&self.second)
        )
    }
}

/// Parses the date/time literal subset used by the native parser/formatter slice.
pub fn parse_datetime_literal(input: &str) -> Result<ParsedDateTime, DateTimeParseError> {
    let (date, rest) = input
        .split_once('T')
        .map_or((input, None), |(date, rest)| (date, Some(rest)));
    let (year, month, day) = parse_date_part(date)?;

    let Some(rest) = rest else {
        return Ok(ParsedDateTime {
            value: DateTime::from_ymd(year, month, day)
                .map_err(|error| DateTimeParseError::new(error.to_string()))?,
            offset_minutes: None,
        });
    };

    let (time, zone) = split_time_and_zone(rest)?;
    let (hour, minute, second) = parse_time_part(time)?;
    let offset_minutes = match zone {
        "" => None,
        suffix => Some(parse_timezone_suffix(suffix)?),
    };

    Ok(ParsedDateTime {
        value: DateTime::from_ymd_hms(year, month, day, hour, minute, second)
            .map_err(|error| DateTimeParseError::new(error.to_string()))?,
        offset_minutes,
    })
}

/// Converts a parsed date/time literal to a target UTC offset and returns qalc text output.
pub fn convert_datetime_literal_to_zone(
    input: &str,
    target: &str,
) -> Result<String, DateTimeParseError> {
    let parsed = parse_datetime_literal(input)?;
    let source_offset = parsed
        .offset_minutes
        .ok_or_else(|| DateTimeParseError::new("source date/time has no timezone"))?;
    let target_offset = parse_timezone_target(target)?;
    let utc = parsed
        .value
        .timestamp_utc()
        .sub(&Number::from_i64(i64::from(source_offset) * 60));
    let shifted = utc.add(&Number::from_i64(i64::from(target_offset) * 60));
    let value = DateTime::from_timestamp_utc(&shifted)
        .map_err(|error| DateTimeParseError::new(error.to_string()))?;

    Ok(format!(
        "\"{}\"",
        format_iso_datetime_with_offset(&value, Some(target_offset))
    ))
}

/// Formats a date/time value with an optional UTC offset suffix.
pub fn format_iso_datetime_with_offset(value: &DateTime, offset_minutes: Option<i32>) -> String {
    let mut out = value.source_string();
    if value.time_is_set {
        if let Some(offset) = offset_minutes {
            out.push_str(&format_timezone_offset(offset));
        }
    }
    out
}

/// Evaluates the focused native date/time parser/formatter expression slice.
pub(crate) fn native_output(expr: &str) -> Result<Option<String>, String> {
    let trimmed = expr.trim();

    if let Some(output) = native_date_arithmetic_output(trimmed)? {
        return Ok(Some(output));
    }
    if let Some(output) = native_datetime_function_output(trimmed)? {
        return Ok(Some(output));
    }

    let Some((lhs, target)) = split_conversion(trimmed) else {
        return Ok(None);
    };

    if target.eq_ignore_ascii_case("time") {
        let Some(seconds) = parse_time_sum(lhs)? else {
            return Ok(None);
        };
        return Ok(Some(format_time_of_day(seconds)));
    }

    if !target.to_ascii_lowercase().starts_with("utc") {
        return Ok(None);
    }

    native_utc_conversion_output(lhs, target)
}

impl PartialEq for DateTime {
    fn eq(&self, other: &Self) -> bool {
        self.year == other.year
            && self.month == other.month
            && self.day == other.day
            && self.hour == other.hour
            && self.minute == other.minute
            && self.second == other.second
            && self.time_is_set == other.time_is_set
    }
}

impl Eq for DateTime {}

impl Hash for DateTime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.year.hash(state);
        self.month.hash(state);
        self.day.hash(state);
        self.hour.hash(state);
        self.minute.hash(state);
        self.second.to_qalc_string().hash(state);
        self.time_is_set.hash(state);
    }
}

impl PartialOrd for DateTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DateTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.year
            .cmp(&other.year)
            .then_with(|| self.month.cmp(&other.month))
            .then_with(|| self.day.cmp(&other.day))
            .then_with(|| self.hour.cmp(&other.hour))
            .then_with(|| self.minute.cmp(&other.minute))
            .then_with(|| {
                self.second
                    .partial_cmp(&other.second)
                    .expect("validated date/time seconds are comparable")
            })
    }
}

fn parse_date_part(input: &str) -> Result<(i64, i64, i64), DateTimeParseError> {
    let mut parts = input.split('-');
    let year = parse_i64_part(parts.next(), "year")?;
    let month = parse_i64_part(parts.next(), "month")?;
    let day = parse_i64_part(parts.next(), "day")?;
    if parts.next().is_some() {
        return Err(DateTimeParseError::new("date has too many fields"));
    }
    Ok((year, month, day))
}

fn split_time_and_zone(input: &str) -> Result<(&str, &str), DateTimeParseError> {
    let time_end = input
        .find(|ch: char| ch == 'Z' || ch == '+' || ch == '-' || ch.is_ascii_alphabetic())
        .unwrap_or(input.len());
    let (time, zone) = input.split_at(time_end);
    if time.is_empty() {
        return Err(DateTimeParseError::new("missing time component"));
    }
    Ok((time, zone))
}

fn parse_time_part(input: &str) -> Result<(i64, i64, Number), DateTimeParseError> {
    let mut parts = input.split(':');
    let hour = parse_i64_part(parts.next(), "hour")?;
    let minute = parse_i64_part(parts.next(), "minute")?;
    let second = match parts.next() {
        Some(second) => second
            .parse::<Number>()
            .map_err(|_| DateTimeParseError::new("invalid second"))?,
        None => Number::new(),
    };
    if parts.next().is_some() {
        return Err(DateTimeParseError::new("time has too many fields"));
    }
    Ok((hour, minute, second))
}

fn parse_i64_part(value: Option<&str>, name: &str) -> Result<i64, DateTimeParseError> {
    value
        .filter(|part| !part.is_empty())
        .ok_or_else(|| DateTimeParseError::new(format!("missing {name}")))?
        .parse::<i64>()
        .map_err(|_| DateTimeParseError::new(format!("invalid {name}")))
}

fn parse_timezone_suffix(input: &str) -> Result<i32, DateTimeParseError> {
    match input {
        "Z" | "UTC" | "utc" => Ok(0),
        "CET" | "cet" => Ok(60),
        suffix if suffix.starts_with('+') || suffix.starts_with('-') => {
            parse_offset_minutes(suffix)
        }
        _ => Err(DateTimeParseError::new(format!("unknown timezone {input}"))),
    }
}

fn parse_timezone_target(input: &str) -> Result<i32, DateTimeParseError> {
    let lower = input.to_ascii_lowercase();
    if lower == "utc" {
        return Ok(0);
    }
    let Some(rest) = lower.strip_prefix("utc") else {
        return Err(DateTimeParseError::new(format!(
            "unsupported timezone target {input}"
        )));
    };
    parse_offset_minutes(rest)
}

fn parse_offset_minutes(input: &str) -> Result<i32, DateTimeParseError> {
    let sign = if input.starts_with('-') { -1 } else { 1 };
    let rest = input
        .strip_prefix(['+', '-'])
        .ok_or_else(|| DateTimeParseError::new("timezone offset must include sign"))?;
    let (hours, minutes) = if let Some((hours, minutes)) = rest.split_once(':') {
        (
            hours
                .parse::<i32>()
                .map_err(|_| DateTimeParseError::new("invalid timezone hour"))?,
            minutes
                .parse::<i32>()
                .map_err(|_| DateTimeParseError::new("invalid timezone minute"))?,
        )
    } else {
        (
            rest.parse::<i32>()
                .map_err(|_| DateTimeParseError::new("invalid timezone hour"))?,
            0,
        )
    };
    if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
        return Err(DateTimeParseError::new("timezone offset out of range"));
    }
    Ok(sign * (hours * 60 + minutes))
}

fn format_timezone_offset(offset_minutes: i32) -> String {
    if offset_minutes == 0 {
        return "Z".to_string();
    }
    let sign = if offset_minutes < 0 { '-' } else { '+' };
    let abs = offset_minutes.abs();
    format!("{sign}{:02}:{:02}", abs / 60, abs % 60)
}

fn split_conversion(input: &str) -> Option<(&str, &str)> {
    let (lhs, target) = input.rsplit_once(" to ")?;
    Some((lhs.trim(), target.trim()))
}

fn unquote(input: &str) -> Option<&str> {
    if input.len() < 2 {
        return None;
    }
    input
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .or_else(|| {
            input
                .strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
        })
}

fn native_date_arithmetic_output(input: &str) -> Result<Option<String>, String> {
    let Some((left, rest)) = parse_leading_quoted(input) else {
        return Ok(None);
    };
    let left = parse_datetime_arg(left)?;
    let rest = rest.trim_start();

    if let Some(days) = rest.strip_prefix('+') {
        let days = parse_day_count(days.trim())?;
        let shifted = left
            .value
            .add_days(&days)
            .map_err(|error| error.to_string())?;
        return Ok(Some(format_quoted_datetime(&shifted, None)));
    }

    if let Some(right) = rest.strip_prefix('-') {
        let Some((right, trailing)) = parse_leading_quoted(right.trim_start()) else {
            return Ok(None);
        };
        if !trailing.trim().is_empty() {
            return Ok(None);
        }
        let right = parse_datetime_arg(right)?;
        let days = right.value.days_to(&left.value);
        return Ok(Some(format_day_duration(&days)));
    }

    Ok(None)
}

fn native_datetime_function_output(input: &str) -> Result<Option<String>, String> {
    if let Some(inner) = strip_function_call(input, "addDays") {
        let Some((date, days)) = inner.split_once(';') else {
            return Err("addDays requires date and day count".to_string());
        };
        let date = parse_datetime_arg(date.trim())?;
        let days = parse_number_literal(days)?;
        let shifted = date
            .value
            .add_days(&days)
            .map_err(|error| error.to_string())?;
        return Ok(Some(format_quoted_datetime(&shifted, None)));
    }

    if let Some(inner) = strip_function_call(input, "timestamp") {
        let date = parse_datetime_arg(inner.trim())?;
        return Ok(Some(parsed_datetime_utc_timestamp(&date).to_qalc_string()));
    }

    if let Some(inner) = strip_function_call(input, "lunarphase") {
        let date = parse_datetime_arg(inner.trim())?;
        return Ok(Some(format!("{:.8}", lunar_phase_fraction(&date))));
    }

    Ok(None)
}

fn native_utc_conversion_output(lhs: &str, target: &str) -> Result<Option<String>, String> {
    let target_offset = parse_timezone_target(target).map_err(|error| error.to_string())?;
    let lhs = lhs.trim();

    if let Some(literal) = unquote(lhs) {
        return convert_datetime_literal_to_zone(literal, target)
            .map(Some)
            .map_err(|error| error.to_string());
    }

    if let Some(inner) = strip_function_call(lhs, "stamptodate") {
        let timestamp = parse_number_literal(inner)?;
        return format_timestamp_to_zone(&timestamp, target_offset).map(Some);
    }

    if let Some(inner) = strip_function_call(lhs, "nextlunarphase") {
        let Some((phase, date)) = inner.split_once(',') else {
            return Err("nextlunarphase requires phase and date".to_string());
        };
        let phase = parse_phase_fraction(phase)?;
        let date = parse_datetime_arg(date.trim())?;
        let value = next_lunar_phase_datetime(&date, phase)?;
        return format_utc_datetime_to_zone(&value, target_offset).map(Some);
    }

    Ok(None)
}

fn parse_leading_quoted(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();
    let quote = input.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let mut escaped = false;
    for (index, ch) in input.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Some((&input[1..index], &input[index + ch.len_utf8()..]));
        }
    }
    None
}

fn strip_function_call<'a>(input: &'a str, name: &str) -> Option<&'a str> {
    let rest = input.strip_prefix(name)?.trim_start();
    rest.strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .map(str::trim)
}

fn parse_datetime_arg(input: &str) -> Result<ParsedDateTime, String> {
    let input = unquote(input.trim()).unwrap_or_else(|| input.trim());
    parse_datetime_literal(input).map_err(|error| error.to_string())
}

fn parse_number_literal(input: &str) -> Result<Number, String> {
    input
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .parse::<Number>()
        .map_err(|_| format!("invalid number {}", input.trim()))
}

fn parse_day_count(input: &str) -> Result<Number, String> {
    let input = input.trim();
    let day_count = input
        .strip_suffix('d')
        .ok_or_else(|| format!("expected day quantity, got {input}"))?;
    parse_number_literal(day_count)
}

fn parse_phase_fraction(input: &str) -> Result<f64, String> {
    let phase = parse_number_literal(input)?.to_f64();
    let phase = if phase > 1.0 { phase / 360.0 } else { phase };
    if !(0.0..1.0).contains(&phase) {
        return Err(format!("invalid lunar phase {}", input.trim()));
    }
    Ok(phase)
}

fn parsed_datetime_utc_timestamp(parsed: &ParsedDateTime) -> Number {
    parsed.value.timestamp_utc().sub(&Number::from_i64(
        i64::from(parsed.offset_minutes.unwrap_or(0)) * 60,
    ))
}

fn format_timestamp_to_zone(timestamp: &Number, offset_minutes: i32) -> Result<String, String> {
    let shifted = timestamp.add(&Number::from_i64(i64::from(offset_minutes) * 60));
    let value = DateTime::from_timestamp_utc(&shifted).map_err(|error| error.to_string())?;
    Ok(format_quoted_datetime(&value, Some(offset_minutes)))
}

fn format_utc_datetime_to_zone(value: &DateTime, offset_minutes: i32) -> Result<String, String> {
    let shifted = value
        .add_seconds(&Number::from_i64(i64::from(offset_minutes) * 60))
        .map_err(|error| error.to_string())?;
    Ok(format_quoted_datetime(&shifted, Some(offset_minutes)))
}

fn format_quoted_datetime(value: &DateTime, offset_minutes: Option<i32>) -> String {
    format!(
        "\"{}\"",
        format_iso_datetime_with_offset(value, offset_minutes)
    )
}

fn format_day_duration(days: &Number) -> String {
    let mut out = days.to_qalc_string();
    if let Some(stripped) = out.strip_prefix('-') {
        out = format!("−{stripped}");
    }
    format!("{out} d")
}

fn parse_time_sum(input: &str) -> Result<Option<i64>, String> {
    let mut total = 0i64;
    for term in input.split('+') {
        let Some(seconds) = parse_time_quantity(term.trim())? else {
            return Ok(None);
        };
        total = total
            .checked_add(seconds)
            .ok_or_else(|| "time sum is out of range".to_string())?;
    }
    Ok(Some(total))
}

fn parse_time_quantity(input: &str) -> Result<Option<i64>, String> {
    if let Some(seconds) = parse_clock_time(input)? {
        return Ok(Some(seconds));
    }
    parse_unit_time(input)
}

fn parse_clock_time(input: &str) -> Result<Option<i64>, String> {
    if !input.contains(':') {
        return Ok(None);
    }
    let (hour, minute, second) = parse_time_part(input).map_err(|error| error.to_string())?;
    let second =
        number_to_i64(&second).ok_or_else(|| "time seconds must be integer".to_string())?;
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || !(0..=59).contains(&second) {
        return Err("time component out of range".to_string());
    }
    Ok(Some(hour * 3_600 + minute * 60 + second))
}

fn parse_unit_time(input: &str) -> Result<Option<i64>, String> {
    let mut total = 0i64;
    let mut matched = false;
    for part in input.split_whitespace() {
        let (value, suffix) = split_number_suffix(part);
        if value.is_empty() || suffix.is_empty() {
            return Ok(None);
        }
        let value = value
            .parse::<i64>()
            .map_err(|_| format!("invalid time quantity {part}"))?;
        let seconds = match suffix {
            "h" | "hr" | "hour" | "hours" => value * 3_600,
            "min" | "minute" | "minutes" => value * 60,
            "s" | "sec" | "second" | "seconds" => value,
            _ => return Ok(None),
        };
        total = total
            .checked_add(seconds)
            .ok_or_else(|| "time quantity is out of range".to_string())?;
        matched = true;
    }
    Ok(matched.then_some(total))
}

fn split_number_suffix(input: &str) -> (&str, &str) {
    let split = input
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(input.len());
    input.split_at(split)
}

fn number_to_i64(value: &Number) -> Option<i64> {
    (0..=60).find(|candidate| value == &Number::from_i64(*candidate))
}

fn format_time_of_day(seconds: i64) -> String {
    let seconds = seconds.rem_euclid(SECONDS_PER_DAY);
    let hour = seconds / 3_600;
    let minute = (seconds % 3_600) / 60;
    let second = seconds % 60;
    if second == 0 {
        format!("{hour:02}:{minute:02}")
    } else {
        format!("{hour:02}:{minute:02}:{second:02}")
    }
}

fn format_second_component(second: &Number) -> String {
    if let Some(second) = number_to_i64(second) {
        return format!("{second:02}");
    }
    let raw = second.to_qalc_string();
    if raw.starts_with("0.") {
        format!("0{raw}")
    } else {
        raw
    }
}

fn lunar_phase_fraction(date: &ParsedDateTime) -> f64 {
    positive_mod(
        lunar_phase_degrees(datetime_to_universal_fixed(date)),
        360.0,
    ) / 360.0
}

fn next_lunar_phase_datetime(date: &ParsedDateTime, phase: f64) -> Result<DateTime, String> {
    let fixed = lunar_phase_at_or_after_degrees(phase * 360.0, datetime_to_universal_fixed(date));
    let timestamp = ((fixed - GREGORIAN_FIXED_UNIX_EPOCH) * SECONDS_PER_DAY as f64).floor();
    if !timestamp.is_finite() {
        return Err("lunar phase timestamp is out of range".to_string());
    }
    DateTime::from_timestamp_utc(&Number::from_i64(timestamp as i64))
        .map_err(|error| error.to_string())
}

fn datetime_to_universal_fixed(date: &ParsedDateTime) -> f64 {
    parsed_datetime_utc_timestamp(date).to_f64() / SECONDS_PER_DAY as f64
        + GREGORIAN_FIXED_UNIX_EPOCH
}

fn fixed_from_gregorian(year: i64, month: i64, day: i64) -> f64 {
    let y = year - 1;
    (y * 365 + y.div_euclid(4) - y.div_euclid(100)
        + y.div_euclid(400)
        + ((367 * month - 362).div_euclid(12))
        + if month > 2 {
            if is_leap_year(year) {
                -1
            } else {
                -2
            }
        } else {
            0
        }
        + day) as f64
}

fn gregorian_year_from_fixed(fixed: f64) -> i64 {
    let days = fixed.floor() as i64 - GREGORIAN_FIXED_UNIX_EPOCH as i64;
    civil_from_days(days).0
}

fn gregorian_date_difference(y1: i64, m1: i64, d1: i64, y2: i64, m2: i64, d2: i64) -> f64 {
    fixed_from_gregorian(y2, m2, d2) - fixed_from_gregorian(y1, m1, d1)
}

fn ephemeris_correction(tee: f64) -> f64 {
    let mut year = gregorian_year_from_fixed(tee.floor()) as f64;
    if !(-500.0..=2150.0).contains(&year) {
        let year2 = ((year - 1820.0) / 100.0).powi(2) * 32.0 - 20.0;
        return (year2 + (2150.0 - year) * 0.5628) / SECONDS_PER_DAY as f64;
    }
    if year < 500.0 {
        year /= 100.0;
        return cal_poly(
            year,
            &[
                10583.6,
                -1014.41,
                33.78311,
                -5.952053,
                -0.1798452,
                0.022174192,
                0.0090316521,
            ],
        ) / SECONDS_PER_DAY as f64;
    }
    if year < 1600.0 {
        year = (year - 1000.0) / 100.0;
        return cal_poly(
            year,
            &[
                1574.2,
                -556.01,
                71.23472,
                0.319781,
                -0.8503463,
                -0.005050998,
                0.0083572073,
            ],
        ) / SECONDS_PER_DAY as f64;
    }
    if year < 1700.0 {
        year -= 1600.0;
        return cal_poly(year, &[120.0, -0.9808, -0.01532, 0.000140272128])
            / SECONDS_PER_DAY as f64;
    }
    if year < 1800.0 {
        year -= 1700.0;
        return cal_poly(
            year,
            &[8.118780842, -0.005092142, 0.003336121, -0.0000266484],
        ) / SECONDS_PER_DAY as f64;
    }
    if year < 1900.0 {
        year = gregorian_date_difference(1900, 1, 1, year as i64, 7, 1) / 36_525.0;
        return cal_poly(
            year,
            &[
                -0.000009, 0.003844, 0.083563, 0.865736, 4.867575, 15.845535, 31.332267, 38.291999,
                28.316289, 11.636204, 2.043794,
            ],
        );
    }
    if year < 1987.0 {
        year = gregorian_date_difference(1900, 1, 1, year as i64, 7, 1) / 36_525.0;
        return cal_poly(
            year,
            &[
                -0.00002, 0.000297, 0.025184, -0.181133, 0.553040, -0.861938, 0.677066, -0.212591,
            ],
        );
    }
    if year < 2006.0 {
        year -= 2000.0;
        return cal_poly(
            year,
            &[
                63.86,
                0.3345,
                -0.060374,
                0.0017275,
                0.000651814,
                0.00002373599,
            ],
        ) / SECONDS_PER_DAY as f64;
    }
    if year <= 2050.0 {
        year -= 2000.0;
        return cal_poly(year, &[62.92, 0.32217, 0.005589]) / SECONDS_PER_DAY as f64;
    }
    let year2 = ((year - 1820.0) / 100.0).powi(2) * 32.0 - 20.0;
    (year2 + (2150.0 - year) * 0.5628) / SECONDS_PER_DAY as f64
}

fn dynamical_from_universal(tee: f64) -> f64 {
    tee + ephemeris_correction(tee)
}

fn universal_from_dynamical(tee: f64) -> f64 {
    tee - ephemeris_correction(tee)
}

fn julian_centuries(tee: f64) -> f64 {
    (dynamical_from_universal(tee) - J2000) / 36_525.0
}

fn nutation(tee: f64) -> f64 {
    let c = julian_centuries(tee);
    -0.004778 * deg_sin(cal_poly(c, &[124.90, -1934.134, 0.002063]))
        - 0.0003667 * deg_sin(cal_poly(c, &[201.11, 72001.5377, 0.00057]))
}

fn aberration(tee: f64) -> f64 {
    0.0000974 * deg_cos(177.63 + 35999.01848 * julian_centuries(tee)) - 0.005575
}

fn solar_longitude(tee: f64) -> f64 {
    const COEFFICIENTS: [f64; 49] = [
        403406.0, 195207.0, 119433.0, 112392.0, 3891.0, 2819.0, 1721.0, 660.0, 350.0, 334.0, 314.0,
        268.0, 242.0, 234.0, 158.0, 132.0, 129.0, 114.0, 99.0, 93.0, 86.0, 78.0, 72.0, 68.0, 64.0,
        46.0, 38.0, 37.0, 32.0, 29.0, 28.0, 27.0, 27.0, 25.0, 24.0, 21.0, 21.0, 20.0, 18.0, 17.0,
        14.0, 13.0, 13.0, 13.0, 12.0, 10.0, 10.0, 10.0, 10.0,
    ];
    const MULTIPLIERS: [f64; 49] = [
        0.9287892,
        35999.1376958,
        35999.4089666,
        35998.7287385,
        71998.20261,
        71998.4403,
        36000.35726,
        71997.4812,
        32964.4678,
        -19.4410,
        445267.1117,
        45036.8840,
        3.1008,
        22518.4434,
        -19.9739,
        65928.9345,
        9038.0293,
        3034.7684,
        33718.148,
        3034.448,
        -2280.773,
        29929.992,
        31556.493,
        149.588,
        9037.750,
        107997.405,
        -4444.176,
        151.771,
        67555.316,
        31556.080,
        -4561.540,
        107996.706,
        1221.655,
        62894.167,
        31437.369,
        14578.298,
        -31931.757,
        34777.243,
        1221.999,
        62894.511,
        -4442.039,
        107997.909,
        119.066,
        16859.071,
        -4.578,
        26895.292,
        -39.127,
        12297.536,
        90073.778,
    ];
    const ADDENDS: [f64; 49] = [
        270.54861, 340.19128, 63.91854, 331.26220, 317.843, 86.631, 240.052, 310.26, 247.23,
        260.87, 297.82, 343.14, 166.79, 81.53, 3.50, 132.75, 182.95, 162.03, 29.8, 266.4, 249.2,
        157.6, 257.8, 185.1, 69.9, 8.0, 197.1, 250.4, 65.3, 162.7, 341.5, 291.6, 98.5, 146.7,
        110.0, 5.2, 342.6, 230.9, 256.1, 45.3, 242.9, 115.2, 151.8, 285.3, 53.3, 126.6, 205.7,
        85.9, 146.1,
    ];

    let c = julian_centuries(tee);
    let series = COEFFICIENTS
        .iter()
        .zip(MULTIPLIERS)
        .zip(ADDENDS)
        .map(|((coefficient, multiplier), addend)| coefficient * deg_sin(addend + multiplier * c))
        .sum::<f64>();

    positive_mod(
        282.7771834
            + 36000.76953744 * c
            + 0.000005729577951308232 * series
            + aberration(tee)
            + nutation(tee),
        360.0,
    )
}

fn mean_lunar_longitude(c: f64) -> f64 {
    positive_mod(
        cal_poly(
            c,
            &[
                218.3164477,
                481267.88123421,
                -0.0015786,
                1.0 / 538841.0,
                -1.0 / 65194000.0,
            ],
        ),
        360.0,
    )
}

fn lunar_elongation(c: f64) -> f64 {
    positive_mod(
        cal_poly(
            c,
            &[
                297.8501921,
                445267.1114034,
                -0.0018819,
                1.0 / 545868.0,
                -1.0 / 113065000.0,
            ],
        ),
        360.0,
    )
}

fn solar_anomaly(c: f64) -> f64 {
    positive_mod(
        cal_poly(
            c,
            &[357.5291092, 35999.0502909, -0.0001536, 1.0 / 24490000.0],
        ),
        360.0,
    )
}

fn lunar_anomaly(c: f64) -> f64 {
    positive_mod(
        cal_poly(
            c,
            &[
                134.9633964,
                477198.8675055,
                0.0087414,
                1.0 / 69699.0,
                -1.0 / 14712000.0,
            ],
        ),
        360.0,
    )
}

fn moon_node(c: f64) -> f64 {
    positive_mod(
        cal_poly(
            c,
            &[
                93.2720950,
                483202.0175233,
                -0.0036539,
                -1.0 / 3526000.0,
                1.0 / 863310000.0,
            ],
        ),
        360.0,
    )
}

fn lunar_longitude(tee: f64) -> f64 {
    const ARG_D: [f64; 59] = [
        0.0, 2.0, 2.0, 0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 2.0, 0.0, 1.0, 0.0, 2.0, 0.0, 0.0, 4.0, 0.0,
        4.0, 2.0, 2.0, 1.0, 1.0, 2.0, 2.0, 4.0, 2.0, 0.0, 2.0, 2.0, 1.0, 2.0, 0.0, 0.0, 2.0, 2.0,
        2.0, 4.0, 0.0, 3.0, 2.0, 4.0, 0.0, 2.0, 2.0, 2.0, 4.0, 0.0, 4.0, 1.0, 2.0, 0.0, 1.0, 3.0,
        4.0, 2.0, 0.0, 1.0, 2.0,
    ];
    const ARG_M: [f64; 59] = [
        0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, -1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 1.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0, -2.0, 1.0, 2.0, -2.0,
        0.0, 0.0, -1.0, 0.0, 0.0, 1.0, -1.0, 2.0, 2.0, 1.0, -1.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0,
        1.0, 0.0, 0.0, -1.0, 2.0, 1.0, 0.0,
    ];
    const ARG_MP: [f64; 59] = [
        1.0, -1.0, 0.0, 2.0, 0.0, 0.0, -2.0, -1.0, 1.0, 0.0, -1.0, 0.0, 1.0, 0.0, 1.0, 1.0, -1.0,
        3.0, -2.0, -1.0, 0.0, -1.0, 0.0, 1.0, 2.0, 0.0, -3.0, -2.0, -1.0, -2.0, 1.0, 0.0, 2.0, 0.0,
        -1.0, 1.0, 0.0, -1.0, 2.0, -1.0, 1.0, -2.0, -1.0, -1.0, -2.0, 0.0, 1.0, 4.0, 0.0, -2.0,
        0.0, 2.0, 1.0, -2.0, -3.0, 2.0, 1.0, -1.0, 3.0,
    ];
    const ARG_F: [f64; 59] = [
        0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -2.0, 2.0, -2.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -2.0,
        2.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -2.0, 0.0, 0.0, 0.0, 0.0, -2.0, -2.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    const SINE_COEFF: [f64; 59] = [
        6288774.0, 1274027.0, 658314.0, 213618.0, -185116.0, -114332.0, 58793.0, 57066.0, 53322.0,
        45758.0, -40923.0, -34720.0, -30383.0, 15327.0, -12528.0, 10980.0, 10675.0, 10034.0,
        8548.0, -7888.0, -6766.0, -5163.0, 4987.0, 4036.0, 3994.0, 3861.0, 3665.0, -2689.0,
        -2602.0, 2390.0, -2348.0, 2236.0, -2120.0, -2069.0, 2048.0, -1773.0, -1595.0, 1215.0,
        -1110.0, -892.0, -810.0, 759.0, -713.0, -700.0, 691.0, 596.0, 549.0, 537.0, 520.0, -487.0,
        -399.0, -381.0, 351.0, -340.0, 330.0, 327.0, -323.0, 299.0, 294.0,
    ];

    let c = julian_centuries(tee);
    let cap_l_prime = mean_lunar_longitude(c);
    let cap_d = lunar_elongation(c);
    let cap_m = solar_anomaly(c);
    let cap_m_prime = lunar_anomaly(c);
    let cap_f = moon_node(c);
    let cap_e = cal_poly(c, &[1.0, -0.002516, -0.0000074]);
    let correction = SINE_COEFF
        .iter()
        .zip(ARG_D)
        .zip(ARG_M)
        .zip(ARG_MP)
        .zip(ARG_F)
        .map(|((((coefficient, d), m), mp), f)| {
            coefficient
                * cap_e.powf(m.abs())
                * deg_sin(d * cap_d + m * cap_m + mp * cap_m_prime + f * cap_f)
        })
        .sum::<f64>()
        * 1e-6;
    let venus = 0.003958 * deg_sin(119.75 + 131.849 * c);
    let jupiter = 0.000318 * deg_sin(53.09 + 479264.29 * c);
    let flat_earth = 0.001962 * deg_sin(cap_l_prime - cap_f);

    positive_mod(
        cap_l_prime + correction + venus + jupiter + flat_earth + nutation(tee),
        360.0,
    )
}

fn nth_new_moon(n: f64) -> f64 {
    const E_FACTOR: [f64; 24] = [
        0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 2.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    const SOLAR_COEFF: [f64; 24] = [
        0.0, 1.0, 0.0, 0.0, -1.0, 1.0, 2.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, -1.0, 2.0, 0.0, 3.0, 1.0,
        0.0, 1.0, -1.0, -1.0, 1.0, 0.0,
    ];
    const LUNAR_COEFF: [f64; 24] = [
        1.0, 0.0, 2.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 2.0, 3.0, 0.0, 0.0, 2.0, 1.0, 2.0, 0.0, 1.0,
        2.0, 1.0, 1.0, 1.0, 3.0, 4.0,
    ];
    const MOON_COEFF: [f64; 24] = [
        0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, -2.0, 2.0, 0.0, 0.0, 2.0, -2.0, 0.0, 0.0, -2.0, 0.0,
        -2.0, 2.0, 2.0, 2.0, -2.0, 0.0, 0.0,
    ];
    const SINE_COEFF: [f64; 24] = [
        -0.40720, 0.17241, 0.01608, 0.01039, 0.00739, -0.00514, 0.00208, -0.00111, -0.00057,
        0.00056, -0.00042, 0.00042, 0.00038, -0.00024, -0.00007, 0.00004, 0.00004, 0.00003,
        0.00003, -0.00003, 0.00003, -0.00002, -0.00002, 0.00002,
    ];
    const ADD_CONST: [f64; 13] = [
        251.88, 251.83, 349.42, 84.66, 141.74, 207.14, 154.84, 34.52, 207.19, 291.34, 161.72,
        239.56, 331.55,
    ];
    const ADD_COEFF: [f64; 13] = [
        0.016321, 26.651886, 36.412478, 18.206239, 53.303771, 2.453732, 7.306860, 27.261239,
        0.121824, 1.844379, 24.198154, 25.513099, 3.592518,
    ];
    const ADD_FACTOR: [f64; 13] = [
        0.000165, 0.000164, 0.000126, 0.000110, 0.000062, 0.000060, 0.000056, 0.000047, 0.000042,
        0.000040, 0.000037, 0.000035, 0.000023,
    ];

    let k = n - 24_724.0;
    let c = k / 1236.85;
    let approx = J2000
        + cal_poly(
            c,
            &[
                5.09766,
                29.530588861 * 1236.85,
                0.00015437,
                -0.000000150,
                0.00000000073,
            ],
        );
    let cap_e = cal_poly(c, &[1.0, -0.002516, -0.0000074]);
    let solar_anomaly_n = cal_poly(c, &[2.5534, 1236.85 * 29.10535670, -0.0000014, -0.00000011]);
    let lunar_anomaly_n = cal_poly(
        c,
        &[
            201.5643,
            385.81693528 * 1236.85,
            0.0107582,
            0.00001238,
            -0.000000058,
        ],
    );
    let moon_argument = cal_poly(
        c,
        &[
            160.7108,
            390.67050284 * 1236.85,
            -0.0016118,
            -0.00000227,
            0.000000011,
        ],
    );
    let cap_omega = cal_poly(c, &[124.7746, -1.56375588 * 1236.85, 0.0020672, 0.00000215]);
    let correction = -0.00017 * deg_sin(cap_omega)
        + SINE_COEFF
            .iter()
            .zip(E_FACTOR)
            .zip(SOLAR_COEFF)
            .zip(LUNAR_COEFF)
            .zip(MOON_COEFF)
            .map(|((((coefficient, e), solar), lunar), moon)| {
                coefficient
                    * cap_e.powf(e)
                    * deg_sin(
                        solar * solar_anomaly_n + lunar * lunar_anomaly_n + moon * moon_argument,
                    )
            })
            .sum::<f64>();
    let extra = 0.000325 * deg_sin(cal_poly(c, &[299.77, 132.8475848, -0.009173]));
    let additional = ADD_CONST
        .iter()
        .zip(ADD_COEFF)
        .zip(ADD_FACTOR)
        .map(|((constant, coefficient), factor)| factor * deg_sin(constant + coefficient * k))
        .sum::<f64>();

    universal_from_dynamical(approx + correction + extra + additional)
}

fn lunar_phase_degrees(tee: f64) -> f64 {
    let phi = positive_mod(lunar_longitude(tee) - solar_longitude(tee), 360.0);
    let t0 = nth_new_moon(0.0);
    let n = ((tee - t0) / MEAN_SYNODIC_MONTH).round();
    let phi_prime = positive_mod((tee - nth_new_moon(n)) / MEAN_SYNODIC_MONTH, 1.0) * 360.0;
    if (phi - phi_prime).abs() > 180.0 {
        phi_prime
    } else {
        phi
    }
}

fn lunar_phase_at_or_after_degrees(phase: f64, tee: f64) -> f64 {
    let rate = MEAN_SYNODIC_MONTH / 360.0;
    let tau = tee + positive_mod(phase - lunar_phase_degrees(tee), 360.0) * rate;
    let mut a = (tau - 5.0).max(tee);
    let mut b = tau + 5.0;
    let mut phase_low = phase - 1e-5;
    let mut phase_high = phase + 1e-5;
    if phase_low < 0.0 {
        phase_low += 360.0;
    }
    if phase_high > 360.0 {
        phase_high -= 360.0;
    }

    for _ in 0..100 {
        let test = a + (b - a) / 2.0;
        let new_phase = lunar_phase_degrees(test);
        if phase_high < phase_low {
            if new_phase >= phase_low || new_phase <= phase_high {
                return test;
            }
        } else if new_phase >= phase_low && new_phase <= phase_high {
            return test;
        }

        if positive_mod(new_phase - phase, 360.0) < 180.0 {
            b = test;
        } else {
            a = test;
        }
    }

    a + (b - a) / 2.0
}

fn cal_poly(x: f64, coefficients: &[f64]) -> f64 {
    coefficients
        .iter()
        .rev()
        .fold(0.0, |acc, coefficient| acc * x + coefficient)
}

fn positive_mod(value: f64, modulus: f64) -> f64 {
    value.rem_euclid(modulus)
}

fn deg_sin(degrees: f64) -> f64 {
    degrees.to_radians().sin()
}

fn deg_cos(degrees: f64) -> f64 {
    degrees.to_radians().cos()
}

/// Returns true when `year` is a Gregorian leap year.
pub fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_per_month(month: i64, year: i64) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 30,
    }
}

fn days_per_year(year: i64) -> u16 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

fn validate_date(year: i64, month: i64, day: i64) -> Result<(), DateTimeError> {
    if !(1..=12).contains(&month) {
        return Err(DateTimeError::InvalidMonth { month });
    }
    if day < 1 || day > i64::from(days_per_month(month, year)) {
        return Err(DateTimeError::InvalidDay { year, month, day });
    }
    if checked_days_from_civil(year, month, day).is_none() {
        return Err(DateTimeError::OutOfRange);
    }
    Ok(())
}

fn validate_time(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: &Number,
) -> Result<(), DateTimeError> {
    if !(0..=23).contains(&hour) {
        return Err(DateTimeError::InvalidHour { hour });
    }
    if !(0..=59).contains(&minute) {
        return Err(DateTimeError::InvalidMinute { minute });
    }

    let Some(seconds) = exact_real_rational(second) else {
        return Err(DateTimeError::InvalidSecond {
            second: second.clone(),
        });
    };
    if !(RugRational::from(0)..=RugRational::from(60)).contains(&seconds) {
        return Err(DateTimeError::InvalidSecond {
            second: second.clone(),
        });
    }
    if seconds == 60 && !is_leap_second_time(year, month, day, hour, minute) {
        return Err(DateTimeError::InvalidSecond {
            second: second.clone(),
        });
    }
    Ok(())
}

fn is_leap_second_time(year: i64, month: i64, day: i64, hour: i64, minute: i64) -> bool {
    if !(LS_FIRST_YEAR..=LS_LAST_YEAR).contains(&year) {
        return false;
    }
    if hour != 23 || minute != 59 {
        return false;
    }
    if !((month == 6 && day == 30) || (month == 12 && day == 31)) {
        return false;
    }
    let index = (year - LS_FIRST_YEAR) * 2 + i64::from(month == 12);
    usize::try_from(index)
        .ok()
        .and_then(|idx| HAS_LEAP_SECOND.get(idx))
        .copied()
        .unwrap_or(false)
}

fn split_truncated_fraction(duration: &Number) -> Result<(i64, Number), DateTimeError> {
    let rational = exact_real_rational(duration).ok_or_else(|| DateTimeError::InvalidDuration {
        duration: duration.clone(),
    })?;
    let whole = trunc_rational_to_integer(&rational);
    let whole_i64 = whole
        .to_i64()
        .ok_or_else(|| DateTimeError::InvalidDuration {
            duration: duration.clone(),
        })?;
    let fraction = rational - RugRational::from(whole);
    Ok((whole_i64, number_from_rational(fraction)))
}

fn exact_real_rational(number: &Number) -> Option<RugRational> {
    if !number.is_real() || number.is_interval() || number.is_nan() || number.is_infinite() {
        return None;
    }
    let (real, imag) = number.to_canonical_real_imag();
    if !imag.is_real_zero() {
        return None;
    }
    match real {
        NumberValue::Rational(rational) => Some(rational.value),
        _ => None,
    }
}

fn number_from_rational(value: RugRational) -> Number {
    Number::from_rational(Rational { value })
}

fn floor_rational_to_integer(value: &RugRational) -> Integer {
    let numerator = value.numer().clone();
    let denominator = value.denom().clone();
    let mut quotient = numerator.clone() / denominator.clone();
    let remainder = numerator % denominator;
    if remainder != 0 && value < &0 {
        quotient -= 1;
    }
    quotient
}

fn trunc_rational_to_integer(value: &RugRational) -> Integer {
    value.numer().clone() / value.denom().clone()
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    checked_days_from_civil(year, month, day).expect("validated date fits day count")
}

fn checked_days_from_civil(year: i64, month: i64, day: i64) -> Option<i64> {
    let year = i128::from(year);
    let month = i128::from(month);
    let day = i128::from(day);
    let adjusted_year = year - i128::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    i64::try_from(era * 146_097 + day_of_era - 719_468).ok()
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = i128::from(days) + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i128::from(month <= 2);
    (
        i64::try_from(year).expect("i64 day count maps to i64 year"),
        i64::try_from(month).expect("month is in 1..=12"),
        i64::try_from(day).expect("day is in 1..=31"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_day_conversion_round_trips_known_dates() {
        for (year, month, day, expected_days) in [
            (1970, 1, 1, 0),
            (2020, 5, 20, 18_402),
            (1969, 12, 31, -1),
            (2000, 2, 29, 11_016),
        ] {
            assert_eq!(days_from_civil(year, month, day), expected_days);
            assert_eq!(civil_from_days(expected_days), (year, month, day));
        }
    }
}
