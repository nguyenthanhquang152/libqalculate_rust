use libqalculate_rust::number::Number;
use libqalculate_rust::parser::operators::parse_expression;
use libqalculate_rust::rates::{
    currency_info, definitions_dir, format_qalc_currency_number, match_currency_conversion,
    RatesCatalog, RatesJsonSnapshot,
};
use std::path::{Path, PathBuf};
use std::str::FromStr;

fn upstream_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../libqalculate/data")
}

#[test]
fn maps_representative_currency_names_and_symbols() {
    assert_eq!(currency_info("EUR"), Some(("EUR", "€")));
    assert_eq!(currency_info("€"), Some(("EUR", "€")));
    assert_eq!(currency_info("USD"), Some(("USD", "$")));
    assert_eq!(currency_info("$"), Some(("USD", "$")));
    assert_eq!(currency_info("JPY"), Some(("JPY", "¥")));
    assert_eq!(currency_info("GBP"), Some(("GBP", "£")));
    assert_eq!(currency_info("BTC"), Some(("BTC", "₿")));
    assert_eq!(currency_info("UNKNOWN"), None);
}

#[test]
fn parses_rates_json_snapshot_with_raw_rate_provenance() {
    let snapshot = RatesJsonSnapshot::load_from_dir(upstream_data_dir()).expect("rates.json loads");

    assert_eq!(snapshot.date(), "2026-05-18");
    assert!(snapshot.source().ends_with("rates.json"));

    for (code, raw) in [
        ("usd", "1.161966"),
        ("jpy", "184.669581"),
        ("btc", "0.000015103978"),
    ] {
        let rate = snapshot
            .rate(code)
            .unwrap_or_else(|| panic!("{code} rate present"));
        assert_eq!(rate.code(), code);
        assert_eq!(rate.provenance().raw(), raw);
        assert!(rate.provenance().source().ends_with("rates.json"));
        assert_eq!(rate.value(), &Number::from_str(raw).expect("rate parses"));
    }
}

#[test]
fn effective_catalog_uses_offline_upstream_source_precedence() {
    let catalog = RatesCatalog::load_from_dir(upstream_data_dir()).expect("rates catalog loads");

    assert_eq!(catalog.effective_date(), Some("2026-05-18"));
    assert_eq!(
        catalog
            .json_snapshot()
            .expect("rates.json snapshot")
            .rate("usd")
            .expect("json usd")
            .provenance()
            .raw(),
        "1.161966"
    );

    let usd = catalog.rate_per_eur("USD").expect("effective USD");
    assert_eq!(usd.code(), "USD");
    assert_eq!(usd.provenance().raw(), "1.1648");
    assert!(usd.provenance().source().ends_with("eurofxref-daily.xml"));

    let jpy = catalog.rate_per_eur("JPY").expect("effective JPY");
    assert_eq!(jpy.provenance().raw(), "184.93");
    assert!(jpy.provenance().source().ends_with("eurofxref-daily.xml"));

    let btc = catalog.rate_per_eur("BTC").expect("effective BTC");
    assert_eq!(btc.provenance().raw(), "66025.7");
    assert!(btc.provenance().source().ends_with("Calculator.cc"));
}

#[test]
fn converts_focused_currency_cases_with_upstream_rate_direction() {
    let catalog = RatesCatalog::load_from_dir(upstream_data_dir()).expect("rates catalog loads");

    let cases = [
        ("1", "EUR", "USD", "1.164800000"),
        ("1", "USD", "EUR", "0.8585164835"),
        ("10", "USD", "EUR", "8.585164835"),
        ("1", "EUR", "JPY", "184.9300000"),
        ("1", "BTC", "EUR", "66025.70000"),
        ("0", "EUR", "USD", "0"),
    ];

    for (amount, source, target, expected) in cases {
        let amount = Number::from_str(amount).expect("amount parses");
        let converted = catalog
            .convert(&amount, source, target)
            .unwrap_or_else(|error| panic!("{source} to {target}: {error}"));
        assert_eq!(
            format_qalc_currency_number(&converted),
            expected,
            "{source} to {target}"
        );
    }
}

#[test]
fn matches_explicit_currency_conversion_ast_only() {
    let parsed = parse_expression("10 USD to EUR").expect("conversion parses");
    let (amount, source, target) =
        match_currency_conversion(&parsed).expect("currency conversion recognized");
    assert_eq!(amount, Number::from_i32(10));
    assert_eq!(source, "USD");
    assert_eq!(target, "EUR");

    let unsupported = parse_expression("10 USD to m").expect("unit conversion parses");
    assert!(match_currency_conversion(&unsupported).is_none());
}

#[test]
fn missing_rates_and_stale_snapshots_are_explicit() {
    let temp = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        temp.path().join("rates.json"),
        r#"{"date":"2020-01-01","eur":{"usd":1.2}}"#,
    )
    .expect("write rates");

    let catalog = RatesCatalog::load_from_dir(temp.path()).expect("minimal catalog loads");
    assert_eq!(catalog.effective_date(), Some("2020-01-01"));
    assert_eq!(catalog.is_stale_as_of("2020-01-10", 7), Some(true));
    assert!(catalog
        .convert(&Number::from_i32(1), "EUR", "JPY")
        .expect_err("JPY is unavailable")
        .contains("JPY"));
}

#[test]
fn default_definitions_dir_points_at_upstream_data_when_env_is_absent() {
    if std::env::var_os("QALCULATE_DEFINITIONS_DIR").is_none() {
        assert!(definitions_dir().ends_with("../libqalculate/data"));
    }
}
