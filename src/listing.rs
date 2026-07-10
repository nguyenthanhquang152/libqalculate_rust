use crate::cli::{DefinitionSelection, ListRequest, ListType};
use libqalculate_rust::datasets::load_dataset_catalog_from_dir;
use libqalculate_rust::definitions::load_definition_xml_file;
use libqalculate_rust::definitions_catalog::FunctionVariableCatalog;
use libqalculate_rust::units::{DefinitionName, PrefixUnitCatalog, UnitType};
use std::path::Path;

const LIST_FOOTER: &str = "For more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).";
const NO_MATCH: &str = "No matching item found.\n\n";

fn name_matches(names: &[DefinitionName], query: &str) -> bool {
    let query_lower = query.to_lowercase();
    for ename in names {
        let name_str = ename.name();
        if ename.case_sensitive {
            if name_str.starts_with(query) {
                return true;
            }
        } else {
            if name_str.to_lowercase().starts_with(&query_lower) {
                return true;
            }
        }
        if query.chars().count() >= 2 {
            let mut start = 0;
            while let Some(pos) = name_str[start..].find('_') {
                let idx = start + pos;
                let sub = &name_str[idx + 1..];
                if sub.chars().count() >= 2 {
                    if ename.case_sensitive {
                        if sub.starts_with(query) {
                            return true;
                        }
                    } else {
                        if sub.to_lowercase().starts_with(&query_lower) {
                            return true;
                        }
                    }
                }
                start = idx + 1;
            }
        }
    }
    false
}

fn title_matches(title: &str, query: &str) -> bool {
    if query.chars().count() < 3 {
        return false;
    }
    let title_lower = title.to_lowercase();
    let query_lower = query.to_lowercase();
    if query_lower.is_empty() {
        return true;
    }
    let mut i = 0;
    let bytes = title_lower.as_bytes();
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'(') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i..].starts_with(query_lower.as_bytes()) {
            return true;
        }
        if let Some(pos) = title_lower[i..].find(' ') {
            i += pos + 1;
        } else {
            break;
        }
    }
    false
}

fn country_matches(countries: &[String], query: &str) -> bool {
    if query.chars().count() < 3 {
        return false;
    }
    let query_lower = query.to_lowercase();
    for country in countries {
        if country.to_lowercase().starts_with(&query_lower) {
            return true;
        }
    }
    false
}

fn format_names(names: &[DefinitionName], unicode_enabled: bool, is_currency: bool) -> String {
    if names.is_empty() {
        return String::new();
    }

    let preferred_idx = if is_currency {
        names
            .iter()
            .enumerate()
            .rposition(|(_, name)| {
                !name.plural
                    && !name.unicode
                    && !name.avoid_input
                    && !name.abbreviation
                    && !name.completion_only
            })
            .unwrap_or(0)
    } else {
        names
            .iter()
            .position(|name| {
                !name.plural
                    && !name.unicode
                    && !name.avoid_input
                    && !name.abbreviation
                    && !name.completion_only
            })
            .unwrap_or(0)
    };

    let ename1 = &names[preferred_idx];
    let mut name_str = ename1.name().to_string();
    for (i, ename) in names.iter().enumerate() {
        if i == preferred_idx {
            continue;
        }
        if !ename.avoid_input
            && !ename.plural
            && (!ename.unicode || unicode_enabled)
            && !ename.completion_only
        {
            name_str += " / ";
            name_str += ename.name();
        }
    }
    name_str
}

pub(crate) fn render_list(
    data_dir: &Path,
    request: &ListRequest,
    selection: &DefinitionSelection,
    unicode_enabled: bool,
) -> Result<String, String> {
    if !selection.global_defs {
        return Ok(NO_MATCH.to_string());
    }

    let search_str = request.search_term.as_deref().unwrap_or("");
    let has_search = !search_str.is_empty();
    let list_all = request.list_type == ListType::All;
    if !has_search && list_all {
        return Ok("\nNo local variables, functions or units have been defined.\n\nFor more information about a specific function, variable, unit, or prefix, please use the info command (in interactive mode).\n\n".to_string());
    }

    let units_enabled = selection.units;
    let currencies_enabled = selection.currencies;

    // Load prefix/unit/currency catalog if needed
    let prefix_unit_catalog = if ((request.list_type == ListType::Prefixes
        || request.list_type == ListType::Units
        || list_all)
        && units_enabled)
        || ((request.list_type == ListType::Units || list_all) && currencies_enabled)
    {
        let mut docs = Vec::new();
        if (request.list_type == ListType::Prefixes
            || request.list_type == ListType::Units
            || list_all)
            && units_enabled
        {
            if request.list_type == ListType::Prefixes || list_all {
                let prefixes_path = data_dir.join("prefixes.xml.in");
                docs.push(
                    load_definition_xml_file(prefixes_path)
                        .map_err(|error| format!("failed to load prefixes.xml.in: {error}"))?,
                );
            }
            if request.list_type == ListType::Units || list_all {
                let units_path = data_dir.join("units.xml.in");
                docs.push(
                    load_definition_xml_file(units_path)
                        .map_err(|error| format!("failed to load units.xml.in: {error}"))?,
                );
            }
        }
        if (request.list_type == ListType::Units || list_all) && currencies_enabled {
            let currencies_path = data_dir.join("currencies.xml.in");
            docs.push(
                load_definition_xml_file(currencies_path)
                    .map_err(|error| format!("failed to load currencies.xml.in: {error}"))?,
            );
        }
        Some(PrefixUnitCatalog::from_documents(docs))
    } else {
        None
    };

    // Load function/variable catalog if needed
    let func_var_catalog = if ((request.list_type == ListType::Functions || list_all)
        && selection.functions)
        || ((request.list_type == ListType::Variables || list_all) && selection.variables)
    {
        let mut docs = Vec::new();
        if (request.list_type == ListType::Functions || list_all) && selection.functions {
            let functions_path = data_dir.join("functions.xml.in");
            docs.push(
                load_definition_xml_file(functions_path)
                    .map_err(|error| format!("failed to load functions.xml.in: {error}"))?,
            );
        }
        if (request.list_type == ListType::Variables || list_all) && selection.variables {
            let variables_path = data_dir.join("variables.xml.in");
            docs.push(
                load_definition_xml_file(variables_path)
                    .map_err(|error| format!("failed to load variables.xml.in: {error}"))?,
            );
        }
        Some(FunctionVariableCatalog::from_documents(docs))
    } else {
        None
    };

    // Load dataset catalog if needed
    let dataset_catalog =
        if (request.list_type == ListType::Functions || list_all) && selection.datasets {
            Some(
                load_dataset_catalog_from_dir(data_dir)
                    .map_err(|error| format!("failed to load datasets: {error}"))?,
            )
        } else {
            None
        };

    let mut name_list = Vec::new();

    if (request.list_type == ListType::Prefixes || list_all) && selection.units {
        if let Some(cat) = &prefix_unit_catalog {
            for prefix in &cat.prefixes().prefixes {
                if prefix.active() {
                    let is_match =
                        search_str.is_empty() || name_matches(prefix.names(), search_str);
                    if is_match {
                        let name_str = format_names(prefix.names(), unicode_enabled, false);
                        name_list.push(name_str);
                        if !search_str.is_empty() {
                            name_list.push(String::new());
                        }
                    }
                }
            }
        }
    }

    if request.list_type == ListType::Functions || list_all {
        if selection.functions {
            if let Some(cat) = &func_var_catalog {
                for func in cat.functions().functions() {
                    if func.active() && !func.hidden() {
                        let is_match = search_str.is_empty()
                            || name_matches(func.names(), search_str)
                            || func.title().is_some_and(|t| title_matches(t, search_str));
                        if is_match {
                            let mut name_str = format_names(func.names(), unicode_enabled, false);
                            if !search_str.is_empty() {
                                if let Some(t) = func.title() {
                                    name_str = format!("{} ({})", name_str, t);
                                }
                            }
                            name_list.push(name_str);
                        }
                    }
                }
            }
        }
        if selection.datasets {
            if let Some(cat) = &dataset_catalog {
                for dataset in cat.datasets() {
                    if dataset.active() && !dataset.hidden() {
                        let is_match = search_str.is_empty()
                            || name_matches(dataset.names(), search_str)
                            || dataset
                                .title()
                                .is_some_and(|t| title_matches(t, search_str));
                        if is_match {
                            let mut name_str =
                                format_names(dataset.names(), unicode_enabled, false);
                            if !search_str.is_empty() {
                                if let Some(t) = dataset.title() {
                                    name_str = format!("{} ({})", name_str, t);
                                }
                            }
                            name_list.push(name_str);
                        }
                    }
                }
            }
        }
    }

    if (request.list_type == ListType::Variables || list_all) && selection.variables {
        if let Some(cat) = &func_var_catalog {
            for var in cat.variables().variables() {
                if var.active() && !var.hidden() {
                    let is_match = search_str.is_empty()
                        || name_matches(var.names(), search_str)
                        || var.title().is_some_and(|t| title_matches(t, search_str));
                    if is_match {
                        let mut name_str = format_names(var.names(), unicode_enabled, false);
                        if !search_str.is_empty() {
                            if let Some(t) = var.title() {
                                name_str = format!("{} ({})", name_str, t);
                            }
                        }
                        name_list.push(name_str);
                    }
                }
            }
        }
    }

    if request.list_type == ListType::Units || list_all {
        if let Some(cat) = &prefix_unit_catalog {
            for unit in &cat.units().units {
                if unit.active() {
                    let is_currency = unit.category_path() == ["Currency"];
                    if unit.hidden() && !is_currency {
                        continue;
                    }
                    let is_enabled = if is_currency {
                        currencies_enabled
                    } else {
                        units_enabled
                    };

                    if is_enabled {
                        let is_composite = unit.kind() == UnitType::Composite;
                        let should_include =
                            !is_composite && (has_search || list_all || !is_currency);

                        if should_include {
                            let is_match = search_str.is_empty()
                                || name_matches(unit.names(), search_str)
                                || unit.title().is_some_and(|t| title_matches(t, search_str))
                                || country_matches(unit.countries(), search_str);

                            if is_match {
                                let mut name_str =
                                    format_names(unit.names(), unicode_enabled, is_currency);
                                if !search_str.is_empty() || is_currency {
                                    if let Some(t) = unit.title() {
                                        name_str = format!("{} ({})", name_str, t);
                                    }
                                }
                                name_list.push(name_str);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(build_columnated_output(name_list))
}

fn build_columnated_output(mut items: Vec<String>) -> String {
    if items.is_empty() {
        return NO_MATCH.to_string();
    }
    items.sort();
    let mut out = String::new();
    let cols = 80;
    // Upstream list_defs uses byte length for the grid width, then Unicode
    // character length for each item's tab padding. Preserve that visible quirk.
    let mut max_l = 0;
    for item in &items {
        if item.len() > max_l {
            max_l = item.len();
        }
    }
    let max_tabs = (max_l / 8) + 1;
    let mut max_c = cols / (max_tabs * 8);
    if max_c == 0 {
        max_c = 1;
    }
    let mut c = 0;
    for item in &items {
        c += 1;
        if c >= max_c {
            c = 0;
            out.push_str(item);
            out.push('\n');
        } else {
            let l = item.chars().count();
            let nr_of_tabs = max_tabs.saturating_sub(l / 8);
            let tabs = "\t".repeat(nr_of_tabs);
            out.push_str(item);
            out.push_str(&tabs);
        }
    }
    if c > 0 {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(LIST_FOOTER);
    out.push_str("\n\n");
    out
}
