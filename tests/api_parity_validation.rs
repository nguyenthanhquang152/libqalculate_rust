use std::fs;

const REQUIRED_HEADERS: &[&str] = &[
    "qalculate.h",
    "Calculator.h",
    "MathStructure.h",
    "Number.h",
    "ExpressionItem.h",
    "Variable.h",
    "Function.h",
    "DataSet.h",
    "Unit.h",
    "Prefix.h",
    "QalculateDateTime.h",
    "includes.h",
];

const REQUIRED_FAMILIES: &[&str] = &[
    "Calculator",
    "MathStructure",
    "Number",
    "ExpressionItem",
    "Variable",
    "Function",
    "DataSet",
    "Unit",
    "Prefix",
    "Date/time",
    "Options and enums",
];

const VALID_STATUSES: &[&str] = &[
    "`native-pass`",
    "`scaffold`",
    "`fallback-only`",
    "`approved-deviation`",
    "`unstarted`",
    "mixed:",
];

#[test]
fn public_api_parity_matrix_classifies_required_headers() {
    let matrix = fs::read_to_string("docs/public_api_parity_matrix.md")
        .expect("docs/public_api_parity_matrix.md should be readable");

    for header in REQUIRED_HEADERS {
        assert!(
            matrix.contains(&format!("`{header}`")),
            "public API parity matrix should classify `{header}`"
        );
    }

    for family in REQUIRED_FAMILIES {
        assert!(
            matrix.contains(family),
            "public API parity matrix should classify family `{family}`"
        );
    }
}

#[test]
fn public_api_parity_matrix_keeps_fallback_distinct_from_native() {
    let matrix = fs::read_to_string("docs/public_api_parity_matrix.md")
        .expect("docs/public_api_parity_matrix.md should be readable");

    assert!(matrix.contains("FFI-only rows"));
    assert!(matrix.contains("cannot be counted as `native-pass`"));
    assert!(matrix.contains("#64"));

    for line in matrix.lines().filter(|line| line.starts_with('|')) {
        let cells = line
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect::<Vec<_>>();
        if cells
            .first()
            .is_some_and(|first| first.parse::<usize>().is_ok())
        {
            assert!(
                VALID_STATUSES.iter().any(|status| line.contains(status)),
                "numbered API matrix row lacks a valid status: {line}"
            );
        }
    }
}
