//! Native offline exchange-rate parsing and focused currency conversion.
//!
//! Upstream oracle:
//! - `../libqalculate/libqalculate/Calculator-definitions.cc::loadExchangeRates`
//! - `../libqalculate/libqalculate/Unit.cc` currency conversion helpers
//! - `../libqalculate/libqalculate/Calculator.cc` built-in BTC relation
//! - `../libqalculate/data/currencies.xml.in`
//! - `../libqalculate/data/eurofxref-daily.xml`
//! - `../libqalculate/data/rates.json`
//!
//! This module deliberately stays offline-only for #49. It does not fetch
//! exchange rates and it does not implement the broader unit conversion engine.

use crate::ast::Expression;
use crate::number::Number;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const BUILTIN_BTC_EUR: &str = "66025.7";

/// Resolves the definitions directory containing upstream data files.
pub fn definitions_dir() -> PathBuf {
    if let Some(value) = std::env::var_os("QALCULATE_DEFINITIONS_DIR") {
        PathBuf::from(value)
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../libqalculate")
            .join("data")
    }
}

/// Helper to get standard ISO code and display symbol for a supported currency.
pub fn currency_info(name: &str) -> Option<(&'static str, &'static str)> {
    match name.to_ascii_lowercase().as_str() {
        "eur" | "€" | "euro" | "euros" => Some(("EUR", "€")),
        "usd" | "$" | "dollar" | "dollars" => Some(("USD", "$")),
        "jpy" | "¥" | "yen" | "yens" => Some(("JPY", "¥")),
        "gbp" | "£" | "pound" | "pounds" => Some(("GBP", "£")),
        "btc" | "₿" | "xbt" | "bitcoin" | "bitcoins" => Some(("BTC", "₿")),
        _ => None,
    }
}

/// Source file and raw text attached to a loaded exchange-rate value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateProvenance {
    source: PathBuf,
    raw: String,
}

impl RateProvenance {
    /// Returns the data source path used for this rate.
    pub fn source(&self) -> &Path {
        &self.source
    }

    /// Returns the raw upstream rate text.
    pub fn raw(&self) -> &str {
        &self.raw
    }
}

/// Parsed rate entry from `rates.json`.
#[derive(Debug, Clone)]
pub struct JsonRate {
    code: String,
    value: Number,
    provenance: RateProvenance,
}

impl JsonRate {
    /// Returns the lowercase currency code.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns the parsed numeric value.
    pub fn value(&self) -> &Number {
        &self.value
    }

    /// Returns source and raw-text provenance.
    pub fn provenance(&self) -> &RateProvenance {
        &self.provenance
    }
}

/// Parsed `rates.json` snapshot.
#[derive(Debug, Clone)]
pub struct RatesJsonSnapshot {
    date: String,
    source: PathBuf,
    rates: HashMap<String, JsonRate>,
}

impl RatesJsonSnapshot {
    /// Loads the `rates.json` snapshot from a definitions directory.
    pub fn load_from_dir(data_dir: impl AsRef<Path>) -> Result<Self, RateLoadError> {
        Self::load_file(data_dir.as_ref().join("rates.json"))
    }

    /// Loads a `rates.json` snapshot from an explicit path.
    pub fn load_file(path: impl AsRef<Path>) -> Result<Self, RateLoadError> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|error| {
            RateLoadError::new(format!("Failed to read {}: {error}", path.display()))
        })?;
        let json: serde_json::Value = serde_json::from_str(&content).map_err(|error| {
            RateLoadError::new(format!("Failed to parse {}: {error}", path.display()))
        })?;

        let date = json
            .get("date")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| RateLoadError::new("rates.json is missing a string date field"))?
            .to_string();
        let eur = json
            .get("eur")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| RateLoadError::new("rates.json is missing an eur object"))?;

        let mut rates = HashMap::new();
        for (code, value) in eur {
            let raw = match value {
                serde_json::Value::Number(number) => number.to_string(),
                serde_json::Value::String(text) => text.clone(),
                _ => continue,
            };
            let parsed = Number::from_str(&raw).map_err(|_| {
                RateLoadError::new(format!("Invalid rate for currency {code}: {raw}"))
            })?;
            let code = code.to_ascii_lowercase();
            rates.insert(
                code.clone(),
                JsonRate {
                    code,
                    value: parsed,
                    provenance: RateProvenance {
                        source: path.to_path_buf(),
                        raw,
                    },
                },
            );
        }

        Ok(Self {
            date,
            source: path.to_path_buf(),
            rates,
        })
    }

    /// Returns the snapshot date string.
    pub fn date(&self) -> &str {
        &self.date
    }

    /// Returns the source file path.
    pub fn source(&self) -> &Path {
        &self.source
    }

    /// Looks up a parsed JSON rate by currency code.
    pub fn rate(&self, code: &str) -> Option<&JsonRate> {
        self.rates.get(&code.to_ascii_lowercase())
    }

    /// Returns all parsed JSON rates.
    pub fn rates(&self) -> &HashMap<String, JsonRate> {
        &self.rates
    }
}

/// Effective offline exchange-rate catalog used by focused native conversion.
#[derive(Debug, Clone)]
pub struct RatesCatalog {
    json: Option<RatesJsonSnapshot>,
    effective_date: Option<String>,
    rates_per_eur: HashMap<String, EffectiveRate>,
}

impl RatesCatalog {
    /// Loads effective offline rates from a definitions directory.
    pub fn load_from_dir(data_dir: impl AsRef<Path>) -> Result<Self, RateLoadError> {
        let data_dir = data_dir.as_ref();
        let json_path = data_dir.join("rates.json");
        let json = if json_path.exists() {
            Some(RatesJsonSnapshot::load_file(&json_path)?)
        } else {
            None
        };
        let mut rates_per_eur = HashMap::new();
        let mut effective_date = json.as_ref().map(|snapshot| snapshot.date().to_string());

        rates_per_eur.insert(
            "eur".to_string(),
            EffectiveRate::new(
                "EUR",
                Number::from_i32(1),
                data_dir.join("currencies.xml.in"),
                "1",
            ),
        );
        if let Some(snapshot) = &json {
            for rate in snapshot.rates().values() {
                rates_per_eur.insert(
                    rate.code().to_string(),
                    EffectiveRate::new(
                        rate.code(),
                        rate.value().clone(),
                        rate.provenance().source().to_path_buf(),
                        rate.provenance().raw(),
                    ),
                );
            }
        }

        // Upstream loads ECB eurofxref rates before JSON and leaves these
        // common fiat currencies at the ECB values when the file is present.
        let eurofxref_path = data_dir.join("eurofxref-daily.xml");
        if eurofxref_path.exists() {
            let ecb = load_eurofxref_rates(&eurofxref_path)?;
            if let Some(date) = ecb.date {
                effective_date = Some(date);
            }
            for rate in ecb.rates {
                rates_per_eur.insert(rate.code.to_ascii_lowercase(), rate);
            }
        }

        // Upstream 5.11.0 initializes BTC as an alias to EUR in Calculator.cc
        // before loading external rate snapshots. That focused behavior is
        // preserved for #49 oracle cases.
        let btc_per_eur = Number::from_i32(1).div(
            &Number::from_str(BUILTIN_BTC_EUR)
                .map_err(|_| RateLoadError::new("Invalid built-in BTC rate"))?,
        );
        rates_per_eur.insert(
            "btc".to_string(),
            EffectiveRate::new(
                "BTC",
                btc_per_eur,
                PathBuf::from("Calculator.cc"),
                BUILTIN_BTC_EUR,
            ),
        );

        Ok(Self {
            json,
            effective_date,
            rates_per_eur,
        })
    }

    /// Returns the parsed `rates.json` snapshot, when present.
    pub fn json_snapshot(&self) -> Option<&RatesJsonSnapshot> {
        self.json.as_ref()
    }

    /// Returns the effective snapshot date used by conversion rates.
    pub fn effective_date(&self) -> Option<&str> {
        self.effective_date.as_deref()
    }

    /// Returns the effective rate of a currency per one EUR.
    pub fn rate_per_eur(&self, code: &str) -> Option<&EffectiveRate> {
        self.rates_per_eur.get(&code.to_ascii_lowercase())
    }

    /// Returns true when the effective snapshot date is older than `max_days`.
    pub fn is_stale_as_of(&self, as_of: &str, max_days: i64) -> Option<bool> {
        let date = self.effective_date()?;
        let snapshot = civil_days(date)?;
        let current = civil_days(as_of)?;
        Some(current - snapshot > max_days)
    }

    /// Performs conversion from source currency to target currency.
    pub fn convert(&self, amount: &Number, source: &str, target: &str) -> Result<Number, String> {
        let src_rate = self
            .rate_per_eur(source)
            .ok_or_else(|| format!("Exchange rate for currency '{source}' is not loaded"))?;
        let tgt_rate = self
            .rate_per_eur(target)
            .ok_or_else(|| format!("Exchange rate for currency '{target}' is not loaded"))?;
        let amount_in_eur = amount.div(src_rate.value());
        Ok(amount_in_eur.mul(tgt_rate.value()))
    }
}

/// Effective exchange rate of one EUR to a currency.
#[derive(Debug, Clone)]
pub struct EffectiveRate {
    code: String,
    value: Number,
    provenance: RateProvenance,
}

impl EffectiveRate {
    fn new(code: &str, value: Number, source: PathBuf, raw: &str) -> Self {
        Self {
            code: code.to_ascii_uppercase(),
            value,
            provenance: RateProvenance {
                source,
                raw: raw.to_string(),
            },
        }
    }

    /// Returns the uppercase currency code.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns the parsed numeric value for one EUR.
    pub fn value(&self) -> &Number {
        &self.value
    }

    /// Returns source and raw-text provenance.
    pub fn provenance(&self) -> &RateProvenance {
        &self.provenance
    }
}

/// Error returned when offline rate data cannot be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLoadError {
    message: String,
}

impl RateLoadError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RateLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RateLoadError {}

/// Formats a native currency conversion number with qalc's default precision.
pub fn format_qalc_currency_number(value: &Number) -> String {
    if value.is_zero() {
        return "0".to_string();
    }
    pad_fixed_decimal_significant_digits(value.to_qalc_string_with_precision(10), 10)
}

/// Match a parsed conversion expression and extract amount/source/target.
pub fn match_currency_conversion(ast: &Expression) -> Option<(Number, String, String)> {
    let Expression::Conversion { expr, target } = ast else {
        return None;
    };
    let (target_code, _) = currency_info(expression_currency_name(target)?)?;

    if let Expression::Multiplication(children) = expr.as_ref() {
        let mut amount = None;
        let mut source = None;
        for child in children.as_slice() {
            match child {
                Expression::Number(number) => amount = Some(number.clone()),
                _ => {
                    if source.is_none() {
                        source = expression_currency_name(child).map(str::to_string);
                    }
                }
            }
        }
        let (source_code, _) = currency_info(source.as_deref()?)?;
        return Some((amount?, source_code.to_string(), target_code.to_string()));
    }

    let (source_code, _) = currency_info(expression_currency_name(expr)?)?;
    Some((
        Number::from_i32(1),
        source_code.to_string(),
        target_code.to_string(),
    ))
}

fn expression_currency_name(expr: &Expression) -> Option<&str> {
    match expr {
        Expression::Symbolic(symbol) => Some(symbol.name()),
        Expression::Unit { unit, .. } => Some(unit.id()),
        _ => None,
    }
}

struct EurofxrefRates {
    date: Option<String>,
    rates: Vec<EffectiveRate>,
}

fn load_eurofxref_rates(path: &Path) -> Result<EurofxrefRates, RateLoadError> {
    let content = fs::read_to_string(path).map_err(|error| {
        RateLoadError::new(format!("Failed to read {}: {error}", path.display()))
    })?;
    let doc = roxmltree::Document::parse(&content).map_err(|error| {
        RateLoadError::new(format!("Failed to parse {}: {error}", path.display()))
    })?;
    let dated_cube = doc
        .descendants()
        .find(|node| node.tag_name().name() == "Cube" && node.attribute("time").is_some());
    let date = dated_cube
        .and_then(|node| node.attribute("time"))
        .map(str::to_string);
    let mut rates = Vec::new();
    for node in doc.descendants() {
        if node.tag_name().name() != "Cube" {
            continue;
        }
        let Some(code) = node.attribute("currency") else {
            continue;
        };
        let Some(raw) = node.attribute("rate") else {
            continue;
        };
        let value = Number::from_str(raw).map_err(|_| {
            RateLoadError::new(format!(
                "Invalid ECB rate for currency {code} in {}: {raw}",
                path.display()
            ))
        })?;
        rates.push(EffectiveRate::new(code, value, path.to_path_buf(), raw));
    }

    Ok(EurofxrefRates { date, rates })
}

fn civil_days(date: &str) -> Option<i64> {
    let mut parts = date.split('-');
    let year = parts.next()?.parse::<i64>().ok()?;
    let month = parts.next()?.parse::<i64>().ok()?;
    let day = parts.next()?.parse::<i64>().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * month_prime + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn pad_fixed_decimal_significant_digits(mut value: String, precision_digits: usize) -> String {
    if value.contains('E') || value.contains('e') || value.contains('∞') || value.contains('/') {
        return value;
    }

    let unsigned = value.strip_prefix('-').unwrap_or(&value);
    let significant = unsigned
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .skip_while(|ch| *ch == '0')
        .count();
    if significant >= precision_digits {
        return value;
    }

    if !value.contains('.') {
        value.push('.');
    }
    value.push_str(&"0".repeat(precision_digits - significant));
    value
}
