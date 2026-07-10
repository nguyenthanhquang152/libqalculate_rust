use super::*;
use proptest::prelude::*;

#[test]
fn parses_upstream_alias_matrix() {
    for alias in &["-v", "-version", "--version"] {
        let invocation = parse_args(vec!["qalc-rs", alias]);
        assert_eq!(invocation.immediate, Some(ImmediateAction::Version));
    }
    for alias in &["-h", "-help", "--help"] {
        let invocation = parse_args(vec!["qalc-rs", alias]);
        assert_eq!(invocation.immediate, Some(ImmediateAction::Help));
    }
    for alias in &["-t", "-terse", "--terse"] {
        let invocation = parse_args(vec!["qalc-rs", alias, "1+1"]);
        assert!(invocation.terse);
        assert_eq!(invocation.expression, Some("1+1".to_string()));
    }
    let invocation = parse_args(vec!["qalc-rs", "-b16", "255"]);
    assert_eq!(invocation.settings, vec!["base 16"]);
    assert_eq!(invocation.expression, Some("255".to_string()));

    let invocation = parse_args(vec!["qalc-rs", "-base", "16", "255"]);
    assert_eq!(invocation.settings, vec!["base 16"]);

    let invocation = parse_args(vec!["qalc-rs", "--definitely-unknown", "--", "1+1"]);
    assert_eq!(
        invocation.diagnostics,
        vec!["Unrecognized option: --definitely-unknown."]
    );
    assert_eq!(invocation.expression, Some("1+1".to_string()));

    let invocation = parse_args(vec!["qalc-rs", "1+1", "-v"]);
    assert!(invocation.immediate.is_none());
    assert_eq!(invocation.expression, Some("1+1 -v".to_string()));
}

#[test]
fn test_raw_u8() {
    let inv = parse_args(vec!["qalc-rs", "-u8"]);
    assert_eq!(inv.unicode, Some(true));
    assert!(inv.settings.is_empty());
    assert!(inv.diagnostics.is_empty());
}

#[test]
fn test_unicode_flags_are_separate_from_deferred_settings() {
    let inv = parse_args(vec!["qalc-rs", "-u8", "-s", "unicode 0"]);
    assert_eq!(inv.settings, vec!["unicode 0"]);
    assert_eq!(inv.unicode, Some(true));

    let inv = parse_args(vec!["qalc-rs", "-s", "unicode 1", "+u8"]);
    assert_eq!(inv.settings, vec!["unicode 1"]);
    assert_eq!(inv.unicode, Some(false));
}

#[test]
fn test_test_file_stops_parsing() {
    let inv = parse_args(vec![
        "qalc-rs",
        "--test-file",
        "FILE",
        "-v",
        "--help",
        "1+1",
    ]);
    assert_eq!(
        inv.command_file,
        Some(CommandFile {
            path: "FILE".to_string(),
            mode: CommandFileMode::Test
        })
    );
    assert_eq!(inv.expression, None);
    assert!(inv.diagnostics.is_empty());
}

#[test]
fn test_test_file_without_path_records_diagnostic() {
    let invocation = parse_args(vec!["qalc-rs", "--test-file"]);
    assert_eq!(invocation.diagnostics, vec!["No file specified."]);
    assert_eq!(
        invocation.command_file,
        Some(CommandFile {
            path: String::new(),
            mode: CommandFileMode::Test,
        })
    );
}

#[test]
fn test_terminal_action_ordering() {
    let inv = parse_args(vec!["qalc-rs", "-v", "-h"]);
    assert_eq!(inv.immediate, Some(ImmediateAction::Version));

    let inv2 = parse_args(vec!["qalc-rs", "-h", "-v"]);
    assert_eq!(inv2.immediate, Some(ImmediateAction::Help));

    let inv3 = parse_args(vec!["qalc-rs", "-i", "-v"]);
    assert_eq!(inv3.immediate, Some(ImmediateAction::Version));

    let inv4 = parse_args(vec!["qalc-rs", "-v", "1+1"]);
    assert_eq!(inv4.immediate, Some(ImmediateAction::Version));
    assert_eq!(inv4.expression, None);
}

#[test]
fn test_timeout_accumulation() {
    let inv = parse_args(vec!["qalc-rs", "-m", "2147483647", "-m", "2147483647"]);
    assert_eq!(inv.timeout_ms, i32::MAX);
}

#[test]
fn test_programming_mode_without_a_base_preserves_upstream_setting_shape() {
    let inv = parse_args(vec!["qalc-rs", "-p"]);
    assert!(inv.programming_mode);
    assert_eq!(inv.settings, vec!["base ", "xor^ 1"]);
}

#[test]
fn test_orthogonal_composition() {
    for args in &[
        vec!["qalc-rs", "-i", "-f", "commands.qalc", "1+1"],
        vec!["qalc-rs", "-f", "commands.qalc", "-i", "1+1"],
    ] {
        let inv = parse_args(args.clone());
        assert!(inv.interactive);
        assert_eq!(
            inv.command_file,
            Some(CommandFile {
                path: "commands.qalc".to_string(),
                mode: CommandFileMode::Commands
            })
        );
        assert_eq!(inv.expression, Some("1+1".to_string()));
    }

    let inv = parse_args(vec!["qalc-rs", "-f", "commands.qalc", "-l"]);
    assert_eq!(
        inv.command_file,
        Some(CommandFile {
            path: "commands.qalc".to_string(),
            mode: CommandFileMode::Commands
        })
    );
    assert_eq!(
        inv.list,
        Some(ListRequest {
            list_type: ListType::All,
            search_term: None
        })
    );

    let inv = parse_args(vec!["qalc-rs", "--list", "sin", "--list-functions"]);
    assert_eq!(
        inv.list,
        Some(ListRequest {
            list_type: ListType::Functions,
            search_term: Some("sin".to_string())
        })
    );

    let inv = parse_args(vec!["qalc-rs", "-p", "16", "+p", "255"]);
    assert!(inv.programming_mode);
    assert_eq!(
        inv.settings,
        vec!["base 16 16", "xor^ 1", "base 10 10", "xor^ 0"]
    );
    assert_eq!(inv.expression, Some("255".to_string()));

    let inv = parse_args(vec!["qalc-rs", "+p", "255"]);
    assert!(!inv.programming_mode);

    let inv = parse_args(vec![
        "qalc-rs",
        "-nounits",
        "-i",
        "--test-file",
        "suite.batch",
        "ignored",
    ]);
    assert!(!inv.interactive);
    assert_eq!(
        inv.command_file,
        Some(CommandFile {
            path: "suite.batch".to_string(),
            mode: CommandFileMode::Test
        })
    );
    assert_eq!(inv.expression, None);
    assert!(inv.terse);
    assert!(inv.defaults);
    assert_eq!(inv.unicode, Some(false));
    assert!(inv.settings.is_empty());
    assert!(!inv.definitions.units);
}

#[test]
fn test_raw_vs_normalized_exactness() {
    for variant in &[
        "-v=1",
        "-h=1",
        "-t=1",
        "-i=1",
        "-e1",
        "-n=1",
        "-u8=1",
        "-nounits=1",
        "--latex=1",
        "--html=1",
    ] {
        let inv = parse_args(vec!["qalc-rs", variant, "--", "1+1"]);
        assert!(
            !inv.diagnostics.is_empty(),
            "expected diagnostic for {}",
            variant
        );
        assert!(inv.immediate.is_none());
        assert!(!inv.interactive);
    }

    let inv = parse_args(vec!["qalc-rs", "-v=1"]);
    assert_eq!(inv.expression, Some("-v=1".to_string()));
    assert!(inv.immediate.is_none());

    let inv = parse_args(vec!["qalc-rs", "-b16"]);
    assert_eq!(inv.settings, vec!["base 16"]);
    let inv = parse_args(vec!["qalc-rs", "-b=16"]);
    assert_eq!(inv.settings, vec!["base 16"]);
    let inv = parse_args(vec!["qalc-rs", "-c0"]);
    assert_eq!(inv.color, ColorMode::Off);
    let inv = parse_args(vec!["qalc-rs", "-c=0"]);
    assert_eq!(inv.color, ColorMode::Off);
    let inv = parse_args(vec!["qalc-rs", "-m100"]);
    assert_eq!(inv.timeout_ms, 100);
    let inv = parse_args(vec!["qalc-rs", "-m=100"]);
    assert_eq!(inv.timeout_ms, 100);
    for variant in ["-b.16", "-m:5"] {
        let inv = parse_args(vec!["qalc-rs", variant, "--", "1+1"]);
        assert_eq!(
            inv.diagnostics,
            vec![format!("Unrecognized option: {variant}.")]
        );
        assert_eq!(inv.expression, Some("1+1".to_string()));
        assert!(inv.settings.is_empty());
        assert_eq!(inv.timeout_ms, 0);

        let inv = parse_args(vec!["qalc-rs", variant]);
        assert_eq!(inv.expression, Some(variant.to_string()));
        assert!(inv.diagnostics.is_empty());
    }
    let inv = parse_args(vec!["qalc-rs", "-p=16"]);
    assert_eq!(inv.settings, vec!["base 16 16", "xor^ 1"]);
    let inv = parse_args(vec!["qalc-rs", "-s=foo"]);
    assert_eq!(inv.settings, vec!["foo"]);
    let inv = parse_args(vec!["qalc-rs", "-f=file"]);
    assert_eq!(
        inv.command_file,
        Some(CommandFile {
            path: "file".to_string(),
            mode: CommandFileMode::Commands
        })
    );
    let inv = parse_args(vec!["qalc-rs", "--list=term"]);
    assert_eq!(
        inv.list,
        Some(ListRequest {
            list_type: ListType::All,
            search_term: Some("term".to_string())
        })
    );
    let inv = parse_args(vec!["qalc-rs", "--test-file=file"]);
    assert_eq!(
        inv.command_file,
        Some(CommandFile {
            path: "file".to_string(),
            mode: CommandFileMode::Test
        })
    );

    for variant in ["-defaults=1", "--defaults=1"] {
        let inv = parse_args(vec!["qalc-rs", variant]);
        assert!(
            inv.defaults,
            "expected {variant} to use normalized matching"
        );
        assert_eq!(inv.expression, None);
        assert!(inv.diagnostics.is_empty());
    }
}

#[test]
fn test_set_omits_only_the_trailing_empty_segment() {
    let inv = parse_args(vec!["qalc-rs", "-s=a;"]);
    assert_eq!(inv.settings, vec!["a"]);

    let inv = parse_args(vec!["qalc-rs", "-s=;"]);
    assert_eq!(inv.settings, vec![""]);

    let inv = parse_args(vec!["qalc-rs", "-s=a;;b"]);
    assert_eq!(inv.settings, vec!["a", "", "b"]);
}

#[test]
fn test_color_values() {
    let inv1 = parse_args(vec!["qalc-rs", "-c=-1"]);
    assert_eq!(inv1.color, ColorMode::Default);
    let inv2 = parse_args(vec!["qalc-rs", "-c=abc"]);
    assert_eq!(inv2.color, ColorMode::Off);
    let inv3 = parse_args(vec!["qalc-rs", "-c=2"]);
    assert_eq!(inv3.color, ColorMode::On);
    let inv4 = parse_args(vec!["qalc-rs", "-c=1suffix"]);
    assert_eq!(inv4.color, ColorMode::On);
    let inv5 = parse_args(vec!["qalc-rs", "-c=-1suffix"]);
    assert_eq!(inv5.color, ColorMode::Default);
    let inv6 = parse_args(vec!["qalc-rs", "-c= 1 suffix"]);
    assert_eq!(inv6.color, ColorMode::On);
    let inv7 = parse_args(vec!["qalc-rs", "-c=0 \t1"]);
    assert_eq!(inv7.color, ColorMode::On);
}

#[test]
fn test_timeout_uses_upstream_decimal_prefix_parsing() {
    let inv = parse_args(vec!["qalc-rs", "-m=100suffix", "-m", " 25 rest"]);
    assert_eq!(inv.timeout_ms, 125);
}

type InvCheckFn = fn(&CliInvocation) -> bool;

#[test]
fn test_full_alias_matrix() {
    // Settings checks
    for (args, expected) in &[
        (vec!["-b16"], vec!["base 16"]),
        (vec!["-b=16"], vec!["base 16"]),
        (vec!["-p", "10"], vec!["base 10 10", "xor^ 1"]),
        (vec!["+p"], vec!["base 10 10", "xor^ 0"]),
        (vec!["-s", "foo 1"], vec!["foo 1"]),
        (vec!["-set", "foo 1"], vec!["foo 1"]),
        (vec!["--set", "foo 1"], vec!["foo 1"]),
    ] {
        let inv = parse_args(std::iter::once("qalc-rs").chain(args.iter().copied()));
        assert_eq!(inv.settings, *expected);
    }

    assert_eq!(parse_args(vec!["qalc-rs", "-u8"]).unicode, Some(true));
    assert_eq!(parse_args(vec!["qalc-rs", "+u8"]).unicode, Some(false));

    // Color checks
    for (args, expected) in &[
        (vec!["-c0"], ColorMode::Off),
        (vec!["-c=0"], ColorMode::Off),
        (vec!["-c1"], ColorMode::On),
        (vec!["-c=1"], ColorMode::On),
        (vec!["--color"], ColorMode::On),
    ] {
        let inv = parse_args(std::iter::once("qalc-rs").chain(args.iter().copied()));
        assert_eq!(inv.color, *expected);
    }

    let inv = parse_args(vec!["qalc-rs", "-c", "0"]);
    assert_eq!(inv.color, ColorMode::On);
    assert_eq!(inv.expression, Some("0".to_string()));

    // Timeout checks
    for (args, expected) in &[
        (vec!["-m", "50"], 50),
        (vec!["-time", "50"], 50),
        (vec!["--time", "50"], 50),
        (vec!["-m100"], 100),
        (vec!["-m=100"], 100),
    ] {
        let inv = parse_args(std::iter::once("qalc-rs").chain(args.iter().copied()));
        assert_eq!(inv.timeout_ms, *expected);
    }

    // Definition toggles
    let toggles: &[(&[&str], InvCheckFn)] = &[
        (&["-nounits"], |inv| !inv.definitions.units),
        (&["--nounits"], |inv| !inv.definitions.units),
        (&["-nocurrencies"], |inv| !inv.definitions.currencies),
        (&["--nocurrencies"], |inv| !inv.definitions.currencies),
        (&["-nofunctions"], |inv| !inv.definitions.functions),
        (&["--nofunctions"], |inv| !inv.definitions.functions),
        (&["-novariables"], |inv| !inv.definitions.variables),
        (&["--novariables"], |inv| !inv.definitions.variables),
        (&["-nodatasets"], |inv| !inv.definitions.datasets),
        (&["--nodatasets"], |inv| !inv.definitions.datasets),
        (&["-nodefs"], |inv| !inv.definitions.global_defs),
        (&["--nodefs"], |inv| !inv.definitions.global_defs),
        (&["-n"], |inv| !inv.definitions.global_defs),
    ];
    for (args, check) in toggles {
        let inv = parse_args(std::iter::once("qalc-rs").chain(args.iter().copied()));
        assert!(check(&inv));
    }

    // Defaults and exrates
    let defaults_exrates: &[(&[&str], InvCheckFn)] = &[
        (&["-defaults"], |inv| inv.defaults),
        (&["--defaults"], |inv| inv.defaults),
        (&["-e"], |inv| inv.exrates),
        (&["-exrates"], |inv| inv.exrates),
        (&["--exrates"], |inv| inv.exrates),
    ];
    for (args, check) in defaults_exrates {
        let inv = parse_args(std::iter::once("qalc-rs").chain(args.iter().copied()));
        assert!(check(&inv));
    }

    // Run files
    for args in &[
        vec!["-f", "file.txt"],
        vec!["-file", "file.txt"],
        vec!["--file", "file.txt"],
    ] {
        let inv = parse_args(std::iter::once("qalc-rs").chain(args.iter().copied()));
        assert_eq!(
            inv.command_file,
            Some(CommandFile {
                path: "file.txt".to_string(),
                mode: CommandFileMode::Commands
            })
        );
    }

    // List options
    for (args, expected_list) in &[
        (
            vec!["-l"],
            ListRequest {
                list_type: ListType::All,
                search_term: None,
            },
        ),
        (
            vec!["-list", "term"],
            ListRequest {
                list_type: ListType::All,
                search_term: Some("term".to_string()),
            },
        ),
        (
            vec!["--list"],
            ListRequest {
                list_type: ListType::All,
                search_term: None,
            },
        ),
        (
            vec!["--list-functions"],
            ListRequest {
                list_type: ListType::Functions,
                search_term: None,
            },
        ),
        (
            vec!["--list-units"],
            ListRequest {
                list_type: ListType::Units,
                search_term: None,
            },
        ),
        (
            vec!["--list-variables"],
            ListRequest {
                list_type: ListType::Variables,
                search_term: None,
            },
        ),
        (
            vec!["--list-prefixes"],
            ListRequest {
                list_type: ListType::Prefixes,
                search_term: None,
            },
        ),
    ] {
        let inv = parse_args(std::iter::once("qalc-rs").chain(args.iter().copied()));
        assert_eq!(inv.list, Some(expected_list.clone()));
    }
}

proptest! {
    #[test]
    fn test_alias_equivalence(
        family_idx in 0..20usize,
        alias_idx in 0..3usize,
    ) {
        struct AliasFamily {
            canonical: &'static str,
            aliases: &'static [&'static str],
            extra_args: &'static [&'static str],
        }
        let families = [
            AliasFamily { canonical: "--help", aliases: &["-h", "-help", "--help"], extra_args: &[] },
            AliasFamily { canonical: "--version", aliases: &["-v", "-V", "-version", "--version"], extra_args: &[] },
            AliasFamily { canonical: "--terse", aliases: &["-t", "-terse", "--terse"], extra_args: &[] },
            AliasFamily { canonical: "--base", aliases: &["-b", "-base", "--base"], extra_args: &["16"] },
            AliasFamily { canonical: "--color", aliases: &["-c", "-color", "--color"], extra_args: &[] },
            AliasFamily { canonical: "--file", aliases: &["-f", "-file", "--file"], extra_args: &["file.txt"] },
            AliasFamily { canonical: "--list", aliases: &["-l", "-list", "--list"], extra_args: &["sin"] },
            AliasFamily { canonical: "--time", aliases: &["-m", "-time", "--time"], extra_args: &["100"] },
            AliasFamily { canonical: "--set", aliases: &["-s", "-set", "--set"], extra_args: &["foo 1"] },
            AliasFamily { canonical: "--nodefs", aliases: &["-n", "-nodefs", "--nodefs"], extra_args: &[] },
            AliasFamily { canonical: "--nounits", aliases: &["-nounits", "--nounits"], extra_args: &[] },
            AliasFamily { canonical: "--nocurrencies", aliases: &["-nocurrencies", "--nocurrencies"], extra_args: &[] },
            AliasFamily { canonical: "--nofunctions", aliases: &["-nofunctions", "--nofunctions"], extra_args: &[] },
            AliasFamily { canonical: "--novariables", aliases: &["-novariables", "--novariables"], extra_args: &[] },
            AliasFamily { canonical: "--nodatasets", aliases: &["-nodatasets", "--nodatasets"], extra_args: &[] },
            AliasFamily { canonical: "--latex", aliases: &["-latex", "--latex"], extra_args: &[] },
            AliasFamily { canonical: "--html", aliases: &["-html", "--html"], extra_args: &[] },
            AliasFamily { canonical: "--defaults", aliases: &["-defaults", "--defaults"], extra_args: &[] },
            AliasFamily { canonical: "--exrates", aliases: &["-e", "-exrates", "--exrates"], extra_args: &[] },
            AliasFamily { canonical: "--interactive", aliases: &["-i", "-interactive", "--interactive"], extra_args: &[] },
        ];

        let family = &families[family_idx];
        let alias = family.aliases[alias_idx % family.aliases.len()];

        let mut canonical_args = vec!["qalc-rs", family.canonical];
        canonical_args.extend(family.extra_args.iter().copied());

        let mut alias_args = vec!["qalc-rs", alias];
        alias_args.extend(family.extra_args.iter().copied());

        let canonical_inv = parse_args(canonical_args);
        let alias_inv = parse_args(alias_args);

        prop_assert_eq!(canonical_inv, alias_inv);
    }

    #[test]
    fn test_separator_preservation(expr_parts in prop::collection::vec("[a-zA-Z0-9+*-/]{1,10}", 1..5)) {
        let mut args = vec!["qalc-rs".to_string(), "--".to_string()];
        args.extend(expr_parts.clone());
        let inv = parse_args(args);
        prop_assert_eq!(inv.expression, Some(expr_parts.join(" ")));
    }

    #[test]
    fn test_totality(args in prop::collection::vec(".*", 0..10)) {
        let mut full_args = vec!["qalc-rs".to_string()];
        full_args.extend(args);
        let _ = parse_args(full_args);
    }
}
