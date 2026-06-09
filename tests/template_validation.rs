use std::fs;

const TEMPLATE_FILES: &[&str] = &[
    "docs/templates/task_packet.md",
    "docs/templates/oracle_evidence.md",
    "docs/templates/pr_closure.md",
    "docs/templates/slop_cleaner_report.md",
];

const REQUIRED_TEMPLATE_FIELDS: &[&str] = &[
    "Upstream References",
    "Acceptance Criteria",
    "Commands Run",
    "Fallback",
    "Review",
    "Deviation",
];

const CONCRETE_UPSTREAM_EXAMPLES: &[&str] = &[
    "Calculator.h",
    "MathStructure.h",
    "Number.h",
    "ExpressionItem.h",
    "Calculator-parse.cc",
    "Calculator-calculate.cc",
    "MathStructure-print.cc",
    "Number.cc",
    "functions.xml.in",
    "units.xml.in",
    "prefixes.xml.in",
    "variables.xml.in",
    "rates.json",
    "parser.batch",
    "operators.batch",
    "numberbase.batch",
    "units.batch",
    "strings.batch",
];

#[test]
fn task_templates_are_linked_from_policy_docs() {
    let agent_skills = fs::read_to_string("docs/agent_skills_mapping.md")
        .expect("docs/agent_skills_mapping.md should be readable");
    let github_workflow = fs::read_to_string("docs/github_workflow.md")
        .expect("docs/github_workflow.md should be readable");

    for template in TEMPLATE_FILES {
        assert!(
            agent_skills.contains(template),
            "{template} should be linked from docs/agent_skills_mapping.md"
        );
        assert!(
            github_workflow.contains(template),
            "{template} should be linked from docs/github_workflow.md"
        );
    }
}

#[test]
fn task_templates_include_required_fields_and_upstream_examples() {
    let combined = TEMPLATE_FILES
        .iter()
        .map(|path| {
            fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("failed to read {path}: {error}"))
        })
        .collect::<Vec<_>>()
        .join("\n");

    for field in REQUIRED_TEMPLATE_FIELDS {
        assert!(
            combined.contains(field),
            "templates should include required field family `{field}`"
        );
    }

    for example in CONCRETE_UPSTREAM_EXAMPLES {
        assert!(
            combined.contains(example),
            "templates should include concrete upstream example `{example}`"
        );
    }

    assert!(
        !combined.to_ascii_lowercase().contains("tbd"),
        "templates should not rely on broad TBD placeholders"
    );
    assert!(
        combined.contains("No C++ fallback") || combined.contains("C++ fallback enabled"),
        "templates should make fallback/native parity distinctions explicit"
    );
}
