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
            self.second.to_qalc_string()
        )
    }
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
