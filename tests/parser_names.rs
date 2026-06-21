//! Parser name-resolution tests for Issue #19.
//!
//! These tests verify that the `NameRegistry` trait and `StaticRegistry`
//! implementation correctly classify names as functions, units, variables,
//! or prefixes, and that combined prefix+unit resolution works.

use libqalculate_rust::ast::DefinitionKind;
use libqalculate_rust::parser::names::{EmptyRegistry, NameMatch, NameRegistry, StaticRegistry};

// ──────────────────────────────────────────────────────────
// EmptyRegistry tests
// ──────────────────────────────────────────────────────────

#[test]
fn empty_registry_resolves_nothing() {
    let reg = EmptyRegistry;
    assert!(reg.lookup("sin", true).is_none());
    assert!(reg.lookup("m", false).is_none());
    assert!(reg.lookup("alpha", false).is_none());
    assert!(reg.lookup("k", false).is_none());
    assert!(reg.lookup("", false).is_none());
}

// ──────────────────────────────────────────────────────────
// StaticRegistry: function matching
// ──────────────────────────────────────────────────────────

#[test]
fn static_registry_resolves_function_with_args() {
    let mut reg = StaticRegistry::new();
    reg.add_function("sin", 1, Some(1));
    reg.add_function("log", 1, Some(2));
    reg.add_function("abs", 1, Some(1));

    match reg.lookup("sin", true) {
        Some(NameMatch::Function {
            definition,
            min_args,
            max_args,
        }) => {
            assert_eq!(definition.id(), "sin");
            assert_eq!(definition.kind(), DefinitionKind::Function);
            assert_eq!(min_args, 1);
            assert_eq!(max_args, Some(1));
        }
        other => panic!("expected Function match, got {other:?}"),
    }

    match reg.lookup("log", true) {
        Some(NameMatch::Function {
            min_args, max_args, ..
        }) => {
            assert_eq!(min_args, 1);
            assert_eq!(max_args, Some(2));
        }
        other => panic!("expected Function match, got {other:?}"),
    }
}

#[test]
fn static_registry_resolves_variadic_function() {
    let mut reg = StaticRegistry::new();
    reg.add_function("concatenate", 1, None);

    match reg.lookup("concatenate", true) {
        Some(NameMatch::Function {
            max_args, min_args, ..
        }) => {
            assert_eq!(min_args, 1);
            assert_eq!(max_args, None);
        }
        other => panic!("expected variadic Function match, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────
// StaticRegistry: unit matching
// ──────────────────────────────────────────────────────────

#[test]
fn static_registry_resolves_unit() {
    let mut reg = StaticRegistry::new();
    reg.add_unit("m");
    reg.add_unit("s");
    reg.add_unit("kg");

    match reg.lookup("m", false) {
        Some(NameMatch::Unit { definition, prefix }) => {
            assert_eq!(definition.id(), "m");
            assert_eq!(definition.kind(), DefinitionKind::Unit);
            assert!(prefix.is_none());
        }
        other => panic!("expected Unit match, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────
// StaticRegistry: variable matching
// ──────────────────────────────────────────────────────────

#[test]
fn static_registry_resolves_variable() {
    let mut reg = StaticRegistry::new();
    reg.add_variable("alpha");
    reg.add_variable("pi");
    reg.add_variable("e");

    match reg.lookup("alpha", false) {
        Some(NameMatch::Variable { definition }) => {
            assert_eq!(definition.id(), "alpha");
            assert_eq!(definition.kind(), DefinitionKind::Variable);
        }
        other => panic!("expected Variable match, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────
// StaticRegistry: prefix matching
// ──────────────────────────────────────────────────────────

#[test]
fn static_registry_resolves_standalone_prefix() {
    let mut reg = StaticRegistry::new();
    reg.add_prefix("mega");

    match reg.lookup("mega", false) {
        Some(NameMatch::Prefix { definition }) => {
            assert_eq!(definition.id(), "mega");
            assert_eq!(definition.kind(), DefinitionKind::Prefix);
        }
        other => panic!("expected Prefix match, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────
// StaticRegistry: prefix + unit combination
// ──────────────────────────────────────────────────────────

#[test]
fn static_registry_resolves_prefix_plus_unit() {
    let mut reg = StaticRegistry::new();
    reg.add_prefix("k");
    reg.add_prefix("M");
    reg.add_prefix("G");
    reg.add_unit("m");
    reg.add_unit("Hz");

    // km → kilo + meter
    match reg.lookup("km", false) {
        Some(NameMatch::Unit { definition, prefix }) => {
            assert_eq!(definition.id(), "m");
            assert_eq!(definition.kind(), DefinitionKind::Unit);
            let p = prefix.expect("expected prefix");
            assert_eq!(p.id(), "k");
            assert_eq!(p.kind(), DefinitionKind::Prefix);
        }
        other => panic!("expected prefixed Unit match for 'km', got {other:?}"),
    }

    // MHz → Mega + Hz
    match reg.lookup("MHz", false) {
        Some(NameMatch::Unit { definition, prefix }) => {
            assert_eq!(definition.id(), "Hz");
            let p = prefix.expect("expected prefix");
            assert_eq!(p.id(), "M");
        }
        other => panic!("expected prefixed Unit match for 'MHz', got {other:?}"),
    }

    // GHz → Giga + Hz
    match reg.lookup("GHz", false) {
        Some(NameMatch::Unit { definition, prefix }) => {
            assert_eq!(definition.id(), "Hz");
            let p = prefix.expect("expected prefix");
            assert_eq!(p.id(), "G");
        }
        other => panic!("expected prefixed Unit match for 'GHz', got {other:?}"),
    }
}

#[test]
fn static_registry_does_not_create_spurious_prefix_unit() {
    let mut reg = StaticRegistry::new();
    reg.add_prefix("k");
    reg.add_unit("m");

    // "kx" should not match: 'x' is not a known unit
    assert!(reg.lookup("kx", false).is_none());

    // "xm" should not match: 'x' is not a known prefix
    assert!(reg.lookup("xm", false).is_none());
}

// ──────────────────────────────────────────────────────────
// StaticRegistry: priority / disambiguation
// ──────────────────────────────────────────────────────────

#[test]
fn function_wins_over_variable_with_same_name() {
    let mut reg = StaticRegistry::new();
    reg.add_variable("sin");
    reg.add_function("sin", 1, Some(1));

    // Per upstream: when followed by `(`, function takes priority
    match reg.lookup("sin", true) {
        Some(NameMatch::Function { .. }) => {} // correct
        other => panic!("expected Function, got {other:?}"),
    }
}

#[test]
fn function_wins_over_unit_with_same_name() {
    let mut reg = StaticRegistry::new();
    reg.add_unit("gamma");
    reg.add_function("gamma", 1, Some(1));

    match reg.lookup("gamma", true) {
        Some(NameMatch::Function { .. }) => {} // correct
        other => panic!("expected Function, got {other:?}"),
    }
}

#[test]
fn unit_wins_over_variable_with_same_name() {
    let mut reg = StaticRegistry::new();
    reg.add_variable("m");
    reg.add_unit("m");

    match reg.lookup("m", false) {
        Some(NameMatch::Unit { .. }) => {} // correct
        other => panic!("expected Unit, got {other:?}"),
    }
}

// ──────────────────────────────────────────────────────────
// StaticRegistry: unknown names
// ──────────────────────────────────────────────────────────

#[test]
fn unknown_name_returns_none() {
    let mut reg = StaticRegistry::new();
    reg.add_function("sin", 1, Some(1));
    reg.add_unit("m");
    reg.add_variable("alpha");
    reg.add_prefix("k");

    assert!(reg.lookup("xyz", false).is_none());
    assert!(reg.lookup("unknown_func", true).is_none());
    assert!(reg.lookup("", false).is_none());
}

// ──────────────────────────────────────────────────────────
// Realistic fixture: upstream data names
// ──────────────────────────────────────────────────────────

/// Build a registry seeded with common upstream names from
/// functions.xml.in, units.xml.in, prefixes.xml.in, variables.xml.in.
fn upstream_test_registry() -> StaticRegistry {
    let mut reg = StaticRegistry::new();

    // Functions (from functions.xml.in)
    reg.add_function("sin", 1, Some(1));
    reg.add_function("cos", 1, Some(1));
    reg.add_function("tan", 1, Some(1));
    reg.add_function("asin", 1, Some(1));
    reg.add_function("acos", 1, Some(1));
    reg.add_function("atan", 1, Some(1));
    reg.add_function("log", 1, Some(2));
    reg.add_function("ln", 1, Some(1));
    reg.add_function("sqrt", 1, Some(1));
    reg.add_function("abs", 1, Some(1));
    reg.add_function("floor", 1, Some(1));
    reg.add_function("ceil", 1, Some(1));
    reg.add_function("round", 1, Some(3));

    // Units (from units.xml.in)
    reg.add_unit("m");
    reg.add_unit("s");
    reg.add_unit("L");
    reg.add_unit("Hz");
    reg.add_unit("V");
    reg.add_unit("A");
    reg.add_unit("W");
    reg.add_unit("ft");
    reg.add_unit("in");
    reg.add_unit("b"); // bit
    reg.add_unit("B"); // byte

    // Prefixes (from prefixes.xml.in)
    reg.add_prefix("k"); // kilo
    reg.add_prefix("M"); // Mega
    reg.add_prefix("G"); // Giga
    reg.add_prefix("T"); // Tera
    reg.add_prefix("m"); // milli — note: same as unit "m"!
    reg.add_prefix("Ki"); // kibi
    reg.add_prefix("Mi"); // mebi
    reg.add_prefix("Gi"); // gibi

    // Variables (from variables.xml.in)
    reg.add_variable("pi");
    reg.add_variable("e");
    reg.add_variable("c"); // speed of light

    reg
}

#[test]
fn upstream_fixture_basic_functions() {
    let reg = upstream_test_registry();

    for func in [
        "sin", "cos", "tan", "asin", "acos", "atan", "ln", "sqrt", "abs",
    ] {
        assert!(
            matches!(reg.lookup(func, true), Some(NameMatch::Function { .. })),
            "expected function match for '{func}'"
        );
    }
}

#[test]
fn upstream_fixture_basic_units() {
    let reg = upstream_test_registry();

    for unit in ["s", "L", "Hz", "V", "A", "W", "ft"] {
        assert!(
            matches!(
                reg.lookup(unit, false),
                Some(NameMatch::Unit { prefix: None, .. })
            ),
            "expected unit match for '{unit}'"
        );
    }
}

#[test]
fn upstream_fixture_prefixed_units() {
    let reg = upstream_test_registry();

    // kHz → k + Hz
    match reg.lookup("kHz", false) {
        Some(NameMatch::Unit { definition, prefix }) => {
            assert_eq!(definition.id(), "Hz");
            assert_eq!(prefix.unwrap().id(), "k");
        }
        other => panic!("expected prefixed unit for 'kHz', got {other:?}"),
    }

    // GiB → Gi + B
    match reg.lookup("GiB", false) {
        Some(NameMatch::Unit { definition, prefix }) => {
            assert_eq!(definition.id(), "B");
            assert_eq!(prefix.unwrap().id(), "Gi");
        }
        other => panic!("expected prefixed unit for 'GiB', got {other:?}"),
    }

    // MHz → M + Hz
    match reg.lookup("MHz", false) {
        Some(NameMatch::Unit { definition, prefix }) => {
            assert_eq!(definition.id(), "Hz");
            assert_eq!(prefix.unwrap().id(), "M");
        }
        other => panic!("expected prefixed unit for 'MHz', got {other:?}"),
    }
}

#[test]
fn upstream_fixture_variables() {
    let reg = upstream_test_registry();

    for var in ["pi", "e", "c"] {
        assert!(
            matches!(reg.lookup(var, false), Some(NameMatch::Variable { .. })),
            "expected variable match for '{var}'"
        );
    }
}

#[test]
fn upstream_fixture_ambiguity_m_is_unit_not_prefix() {
    let reg = upstream_test_registry();

    // When "m" is looked up as a standalone name (not prefix+unit combo),
    // the unit "m" (meter) should win over prefix "m" (milli) because
    // units have higher priority than prefixes in the priority system.
    match reg.lookup("m", false) {
        Some(NameMatch::Unit { definition, prefix }) => {
            assert_eq!(definition.id(), "m");
            assert!(prefix.is_none(), "standalone 'm' should not have a prefix");
        }
        other => panic!("expected Unit match for standalone 'm', got {other:?}"),
    }
}

#[test]
fn upstream_fixture_unknown_names_are_none() {
    let reg = upstream_test_registry();

    assert!(reg.lookup("alpha", false).is_none());
    assert!(reg.lookup("unknown_function", true).is_none());
    assert!(reg.lookup("xyz", false).is_none());
}
