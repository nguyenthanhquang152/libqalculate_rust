//! Typed function and variable definition catalogs loaded from definitions.
//!
//! This module preserves XML-defined function and variable metadata: names,
//! aliases, categories, active/hidden flags, argument metadata, raw expressions,
//! subfunctions, examples, descriptions, values, units, uncertainty, precision,
//! approximation flags, and source provenance. It does not implement function
//! bodies or full variable evaluation semantics.

use crate::definitions::{
    load_definition_xml_file, load_definition_xml_str, DefinitionDiagnostic, DefinitionDocument,
    DefinitionField, DefinitionIoError, DefinitionItem, DefinitionItemKind, DefinitionProvenance,
    DefinitionSource,
};
use crate::parser::names::StaticRegistry;
use std::path::Path;

/// Public definition-name record used by functions and variables.
pub type DefinitionName = crate::units::DefinitionName;

/// Machine-readable warning category emitted while building typed data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildDiagnosticKind {
    /// A function XML child field is preserved by the generic loader but not modeled here.
    UnsupportedFunctionField,
    /// A variable XML child field is preserved by the generic loader but not modeled here.
    UnsupportedVariableField,
}

/// Category of function definition loaded from XML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKind {
    /// Built-in function metadata whose implementation is native Rust or still pending.
    Builtin,
    /// User-defined function expression loaded from XML.
    User,
}

/// Category of variable definition loaded from XML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableKind {
    /// Built-in variable metadata whose value is provided natively.
    Builtin,
    /// Known variable with an XML-provided value expression.
    Known,
    /// Unknown variable placeholder.
    Unknown,
}

/// Structured warning emitted while building typed function/variable data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildDiagnostic {
    kind: BuildDiagnosticKind,
    message: String,
    provenance: DefinitionProvenance,
    tag: Option<String>,
    name: Option<String>,
}

impl BuildDiagnostic {
    fn unsupported_function_field(
        message: String,
        provenance: &DefinitionProvenance,
        tag: Option<String>,
        name: Option<String>,
    ) -> Self {
        Self {
            kind: BuildDiagnosticKind::UnsupportedFunctionField,
            message,
            provenance: provenance.clone(),
            tag,
            name,
        }
    }

    fn unsupported_variable_field(
        message: String,
        provenance: &DefinitionProvenance,
        tag: Option<String>,
        name: Option<String>,
    ) -> Self {
        Self {
            kind: BuildDiagnosticKind::UnsupportedVariableField,
            message,
            provenance: provenance.clone(),
            tag,
            name,
        }
    }

    /// Returns the machine-readable diagnostic category.
    pub fn kind(&self) -> BuildDiagnosticKind {
        self.kind
    }

    /// Returns the human-readable diagnostic message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns source provenance for the XML node that triggered this diagnostic.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }

    /// Returns the XML tag tied to this diagnostic, if one exists.
    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    /// Returns the parsed definition name tied to this diagnostic, if one exists.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

/// A parsed function argument definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionArgument {
    index: usize,
    argument_type: Option<String>,
    title: Option<String>,
    min: Option<String>,
    max: Option<String>,
    condition: Option<String>,
    test: Option<bool>,
    handle_vector: Option<bool>,
    complex_allowed: Option<bool>,
    zero_forbidden: Option<bool>,
}

impl FunctionArgument {
    /// Returns the one-based upstream argument index.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the upstream argument type constraint, if present.
    pub fn argument_type(&self) -> Option<&str> {
        self.argument_type.as_deref()
    }

    /// Returns the human-readable argument title, if present.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns the raw minimum value constraint, if present.
    pub fn min(&self) -> Option<&str> {
        self.min.as_deref()
    }

    /// Returns the raw maximum value constraint, if present.
    pub fn max(&self) -> Option<&str> {
        self.max.as_deref()
    }

    /// Returns the raw argument condition expression, if present.
    pub fn condition(&self) -> Option<&str> {
        self.condition.as_deref()
    }

    /// Returns whether upstream should test this argument before calculation, if specified.
    pub fn test(&self) -> Option<bool> {
        self.test
    }

    /// Returns whether upstream vector handling is enabled for this argument, if specified.
    pub fn handle_vector(&self) -> Option<bool> {
        self.handle_vector
    }

    /// Returns whether upstream allows complex values for this argument, if specified.
    pub fn complex_allowed(&self) -> Option<bool> {
        self.complex_allowed
    }

    /// Returns whether upstream rejects zero values for this argument, if specified.
    pub fn zero_forbidden(&self) -> Option<bool> {
        self.zero_forbidden
    }
}

/// A parsed function definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDefinition {
    kind: FunctionKind,
    names: Vec<DefinitionName>,
    title: Option<String>,
    description: Option<String>,
    expression: Option<String>,
    examples: Vec<String>,
    category_path: Vec<String>,
    active: bool,
    hidden: bool,
    provenance: DefinitionProvenance,
    arguments: Vec<FunctionArgument>,
    conditions: Vec<String>,
    subfunctions: Vec<String>,
}

impl FunctionDefinition {
    /// Returns whether this is built-in metadata or a user-defined XML function.
    pub fn kind(&self) -> FunctionKind {
        self.kind
    }

    /// Returns all names and aliases attached to this function.
    pub fn names(&self) -> &[DefinitionName] {
        &self.names
    }

    /// Returns the human-readable function title, if present.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns the longer human-readable function description, if present.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the raw XML expression for user-defined functions, if present.
    pub fn expression(&self) -> Option<&str> {
        self.expression.as_deref()
    }

    /// Returns example expressions attached to this function.
    pub fn examples(&self) -> &[String] {
        &self.examples
    }

    /// Returns the nested upstream category path for this function.
    pub fn category_path(&self) -> &[String] {
        &self.category_path
    }

    /// Returns whether this function is active.
    pub fn active(&self) -> bool {
        self.active
    }

    /// Returns whether this function is hidden upstream.
    pub fn hidden(&self) -> bool {
        self.hidden
    }

    /// Returns XML source provenance for this function.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }

    /// Returns argument metadata in upstream order.
    pub fn arguments(&self) -> &[FunctionArgument] {
        &self.arguments
    }

    /// Returns raw function condition expressions attached to this definition.
    pub fn conditions(&self) -> &[String] {
        &self.conditions
    }

    /// Returns raw subfunction expressions attached to this definition.
    pub fn subfunctions(&self) -> &[String] {
        &self.subfunctions
    }

    /// Returns the currently loaded minimum arity.
    pub fn min_args(&self) -> usize {
        self.arguments.len()
    }

    /// Returns the currently loaded maximum arity, if known.
    pub fn max_args(&self) -> Option<usize> {
        Some(self.arguments.len())
    }
}

/// A parsed variable definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableDefinition {
    kind: VariableKind,
    names: Vec<DefinitionName>,
    title: Option<String>,
    description: Option<String>,
    value: Option<String>,
    unit: Option<String>,
    uncertainty: Option<String>,
    uncertainty_is_relative: bool,
    precision: Option<String>,
    approximate: bool,
    category_path: Vec<String>,
    active: bool,
    hidden: bool,
    provenance: DefinitionProvenance,
}

impl VariableDefinition {
    /// Returns whether this is built-in metadata, a known XML value, or an unknown variable.
    pub fn kind(&self) -> VariableKind {
        self.kind
    }

    /// Returns all names and aliases attached to this variable.
    pub fn names(&self) -> &[DefinitionName] {
        &self.names
    }

    /// Returns the human-readable variable title, if present.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns the longer human-readable variable description, if present.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the raw XML value expression, if present.
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    /// Returns the unit attribute attached to the XML value, if present.
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }

    /// Returns the uncertainty attribute attached to the XML value, if present.
    pub fn uncertainty(&self) -> Option<&str> {
        self.uncertainty.as_deref()
    }

    /// Returns whether the stored uncertainty attribute is relative.
    pub fn uncertainty_is_relative(&self) -> bool {
        self.uncertainty_is_relative
    }

    /// Returns the raw precision attribute attached to the XML value, if present.
    pub fn precision(&self) -> Option<&str> {
        self.precision.as_deref()
    }

    /// Returns whether the XML value is marked approximate.
    pub fn approximate(&self) -> bool {
        self.approximate
    }

    /// Returns the nested upstream category path for this variable.
    pub fn category_path(&self) -> &[String] {
        &self.category_path
    }

    /// Returns whether this variable is active.
    pub fn active(&self) -> bool {
        self.active
    }

    /// Returns whether this variable is hidden upstream.
    pub fn hidden(&self) -> bool {
        self.hidden
    }

    /// Returns XML source provenance for this variable.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }
}

/// Catalog of loaded function definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCatalog {
    functions: Vec<FunctionDefinition>,
}

impl FunctionCatalog {
    /// Builds a function catalog from a generic document while collecting builder warnings.
    pub fn from_document_with_diagnostics(
        document: &DefinitionDocument,
        diagnostics: &mut Vec<BuildDiagnostic>,
    ) -> Self {
        let functions = document
            .items()
            .iter()
            .filter(|item| {
                matches!(
                    item.kind(),
                    DefinitionItemKind::BuiltinFunction | DefinitionItemKind::Function
                )
            })
            .map(|item| parse_function_definition(item, diagnostics))
            .collect();

        Self { functions }
    }

    /// Returns all loaded function definitions.
    pub fn functions(&self) -> &[FunctionDefinition] {
        &self.functions
    }

    /// Finds a function definition by any of its loaded names.
    pub fn find_by_name(&self, name: &str) -> Option<&FunctionDefinition> {
        self.functions.iter().find(|function| {
            function
                .names
                .iter()
                .any(|candidate| candidate.name() == name)
        })
    }

    /// Registers all active function names into the static name registry.
    pub fn register_all(&self, registry: &mut StaticRegistry) {
        for function in &self.functions {
            if function.active {
                for name in &function.names {
                    registry.add_function(name.name(), function.min_args(), function.max_args());
                }
            }
        }
    }
}

/// Catalog of loaded variable definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableCatalog {
    variables: Vec<VariableDefinition>,
}

impl VariableCatalog {
    /// Builds a variable catalog from a generic document while collecting builder warnings.
    pub fn from_document_with_diagnostics(
        document: &DefinitionDocument,
        diagnostics: &mut Vec<BuildDiagnostic>,
    ) -> Self {
        let variables = document
            .items()
            .iter()
            .filter(|item| {
                matches!(
                    item.kind(),
                    DefinitionItemKind::BuiltinVariable
                        | DefinitionItemKind::Variable
                        | DefinitionItemKind::UnknownVariable
                )
            })
            .map(|item| parse_variable_definition(item, diagnostics))
            .collect();

        Self { variables }
    }

    /// Returns all loaded variable definitions.
    pub fn variables(&self) -> &[VariableDefinition] {
        &self.variables
    }

    /// Finds a variable definition by any of its loaded names.
    pub fn find_by_name(&self, name: &str) -> Option<&VariableDefinition> {
        self.variables.iter().find(|variable| {
            variable
                .names
                .iter()
                .any(|candidate| candidate.name() == name)
        })
    }

    /// Registers all active variable names into the static name registry.
    pub fn register_all(&self, registry: &mut StaticRegistry) {
        for variable in &self.variables {
            if variable.active {
                for name in &variable.names {
                    registry.add_variable(name.name());
                }
            }
        }
    }
}

/// Combined function and variable catalog loaded from upstream definition XML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionVariableCatalog {
    functions: FunctionCatalog,
    variables: VariableCatalog,
    source_diagnostics: Vec<DefinitionDiagnostic>,
    diagnostics: Vec<BuildDiagnostic>,
}

impl FunctionVariableCatalog {
    /// Builds a combined catalog from already-loaded generic definition documents.
    pub fn from_documents<I>(documents: I) -> Self
    where
        I: IntoIterator<Item = DefinitionDocument>,
    {
        let documents = documents.into_iter().collect::<Vec<_>>();
        let mut diagnostics = Vec::new();
        let mut source_diagnostics = Vec::new();
        let mut functions = Vec::new();
        let mut variables = Vec::new();

        for document in &documents {
            source_diagnostics.extend(document.diagnostics().iter().cloned());

            let mut function_catalog =
                FunctionCatalog::from_document_with_diagnostics(document, &mut diagnostics);
            functions.append(&mut function_catalog.functions);

            let mut variable_catalog =
                VariableCatalog::from_document_with_diagnostics(document, &mut diagnostics);
            variables.append(&mut variable_catalog.variables);
        }

        Self {
            functions: FunctionCatalog { functions },
            variables: VariableCatalog { variables },
            source_diagnostics,
            diagnostics,
        }
    }

    /// Builds a combined catalog from named XML string sources.
    pub fn from_xml_sources<I, S, X>(sources: I) -> Self
    where
        I: IntoIterator<Item = (S, X)>,
        S: Into<DefinitionSource>,
        X: AsRef<str>,
    {
        let documents = sources
            .into_iter()
            .map(|(source, xml)| load_definition_xml_str(source, xml.as_ref()));
        Self::from_documents(documents)
    }

    /// Returns the loaded function catalog.
    pub fn functions(&self) -> &FunctionCatalog {
        &self.functions
    }

    /// Returns the loaded variable catalog.
    pub fn variables(&self) -> &VariableCatalog {
        &self.variables
    }

    /// Returns recoverable typed-builder diagnostics.
    pub fn diagnostics(&self) -> &[BuildDiagnostic] {
        &self.diagnostics
    }

    /// Returns diagnostics emitted by the generic XML definition loader.
    pub fn source_diagnostics(&self) -> &[DefinitionDiagnostic] {
        &self.source_diagnostics
    }

    /// Finds a function definition by any of its loaded names.
    pub fn function_by_name(&self, name: &str) -> Option<&FunctionDefinition> {
        self.functions.find_by_name(name)
    }

    /// Finds a variable definition by any of its loaded names.
    pub fn variable_by_name(&self, name: &str) -> Option<&VariableDefinition> {
        self.variables.find_by_name(name)
    }

    /// Registers all active loaded function and variable names into a parser registry.
    pub fn register_into(&self, registry: &mut StaticRegistry) {
        self.functions.register_all(registry);
        self.variables.register_all(registry);
    }
}

/// Loads the upstream function and variable XML files from a data directory.
pub fn load_function_variable_catalog_from_dir(
    data_dir: impl AsRef<Path>,
) -> Result<FunctionVariableCatalog, DefinitionIoError> {
    let data_dir = data_dir.as_ref();
    let functions = load_definition_xml_file(data_dir.join("functions.xml.in"))?;
    let variables = load_definition_xml_file(data_dir.join("variables.xml.in"))?;
    Ok(FunctionVariableCatalog::from_documents([
        functions, variables,
    ]))
}

fn parse_function_definition(
    item: &DefinitionItem,
    diagnostics: &mut Vec<BuildDiagnostic>,
) -> FunctionDefinition {
    let kind = match item.kind() {
        DefinitionItemKind::BuiltinFunction => FunctionKind::Builtin,
        DefinitionItemKind::Function => FunctionKind::User,
        _ => unreachable!("function parser called for non-function item"),
    };
    let names = parse_names_with_field_flags(item);
    let mut title = None;
    let mut description = None;
    let mut expression = None;
    let mut examples = Vec::new();
    let mut hidden = false;
    let mut arguments = Vec::new();
    let mut conditions = Vec::new();
    let mut subfunctions = Vec::new();

    for field in item.fields() {
        match field.tag() {
            "title" => title = field.text().map(clean_translated_label),
            "description" => description = field.text().map(clean_translated_label),
            "expression" => expression = field.text().map(ToOwned::to_owned),
            "example" => {
                if let Some(text) = field.text() {
                    examples.push(clean_translated_label(text));
                }
            }
            "argument" => arguments.push(parse_function_argument(field, arguments.len() + 1)),
            "condition" => {
                if let Some(text) = field.text() {
                    conditions.push(text.to_string());
                }
            }
            "subfunction" => {
                if let Some(text) = field.text() {
                    subfunctions.push(text.to_string());
                }
            }
            "hidden" => hidden = field.text() == Some("true"),
            "names" | "name" | "abbreviation" | "plural" => {}
            other => diagnostics.push(BuildDiagnostic::unsupported_function_field(
                format!("Unsupported function field <{other}>"),
                field.provenance(),
                Some(other.to_string()),
                item.names().first().cloned(),
            )),
        }
    }

    FunctionDefinition {
        kind,
        names,
        title,
        description,
        expression,
        examples,
        category_path: item.category_path().to_vec(),
        active: item.active(),
        hidden,
        provenance: item.provenance().clone(),
        arguments,
        conditions,
        subfunctions,
    }
}

fn parse_function_argument(field: &DefinitionField, fallback_index: usize) -> FunctionArgument {
    let index = field
        .attributes()
        .get("index")
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback_index);
    let argument_type = field.attributes().get("type").cloned();
    let mut title = None;
    let mut min = None;
    let mut max = None;
    let mut condition = None;
    let mut test = None;
    let mut handle_vector = None;
    let mut complex_allowed = None;
    let mut zero_forbidden = None;

    for child in field.fields() {
        match child.tag() {
            "title" => title = child.text().map(clean_translated_label),
            "min" => min = child.text().map(ToOwned::to_owned),
            "max" => max = child.text().map(ToOwned::to_owned),
            "condition" => condition = child.text().map(ToOwned::to_owned),
            "test" => test = child.text().and_then(parse_bool),
            "handle_vector" => handle_vector = child.text().and_then(parse_bool),
            "complex_allowed" => complex_allowed = child.text().and_then(parse_bool),
            "zero_forbidden" => zero_forbidden = child.text().and_then(parse_bool),
            _ => {}
        }
    }

    FunctionArgument {
        index,
        argument_type,
        title,
        min,
        max,
        condition,
        test,
        handle_vector,
        complex_allowed,
        zero_forbidden,
    }
}

fn parse_variable_definition(
    item: &DefinitionItem,
    diagnostics: &mut Vec<BuildDiagnostic>,
) -> VariableDefinition {
    let kind = match item.kind() {
        DefinitionItemKind::BuiltinVariable => VariableKind::Builtin,
        DefinitionItemKind::Variable => VariableKind::Known,
        DefinitionItemKind::UnknownVariable => VariableKind::Unknown,
        _ => unreachable!("variable parser called for non-variable item"),
    };
    let names = parse_names_with_field_flags(item);
    let mut title = None;
    let mut description = None;
    let mut value = None;
    let mut unit = None;
    let mut uncertainty = None;
    let mut uncertainty_is_relative = false;
    let mut precision = None;
    let mut approximate = false;
    let mut hidden = false;

    for field in item.fields() {
        match field.tag() {
            "title" => title = field.text().map(clean_translated_label),
            "description" => description = field.text().map(clean_translated_label),
            "value" => {
                value = field.text().map(ToOwned::to_owned);
                unit = field.attributes().get("unit").cloned();
                if let Some(relative_uncertainty) = field.attributes().get("relative_uncertainty") {
                    uncertainty = Some(relative_uncertainty.clone());
                    uncertainty_is_relative = true;
                } else {
                    uncertainty = field.attributes().get("uncertainty").cloned();
                    uncertainty_is_relative = false;
                }
                precision = field.attributes().get("precision").cloned();
                approximate = parse_approximate_attribute(field);
            }
            "hidden" => hidden = field.text() == Some("true"),
            "names" | "name" | "abbreviation" | "plural" => {}
            other => diagnostics.push(BuildDiagnostic::unsupported_variable_field(
                format!("Unsupported variable field <{other}>"),
                field.provenance(),
                Some(other.to_string()),
                item.names().first().cloned(),
            )),
        }
    }

    VariableDefinition {
        kind,
        names,
        title,
        description,
        value,
        unit,
        uncertainty,
        uncertainty_is_relative,
        precision,
        approximate,
        category_path: item.category_path().to_vec(),
        active: item.active(),
        hidden,
        provenance: item.provenance().clone(),
    }
}

fn clean_translated_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with('!') {
        return trimmed.to_string();
    }
    match trimmed[1..].find('!') {
        Some(index) => trimmed[index + 2..].to_string(),
        None => trimmed.to_string(),
    }
}

fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

fn parse_approximate_attribute(field: &DefinitionField) -> bool {
    if let Some(value) = field.attributes().get("approximate") {
        return value == "true";
    }
    if let Some(value) = field.attributes().get("precise") {
        return value != "true";
    }
    false
}

fn parse_names_with_field_flags(item: &DefinitionItem) -> Vec<DefinitionName> {
    let mut names = Vec::new();

    for field in item.fields() {
        match field.tag() {
            "names" => {
                if let Some(text) = field.text() {
                    names.extend(text.split(',').map(DefinitionName::parse));
                }
            }
            "name" => {
                if let Some(text) = field.text() {
                    names.push(DefinitionName::parse(text));
                }
            }
            "abbreviation" | "plural" => {
                if let Some(text) = field.text() {
                    let mut name = DefinitionName::parse(text);
                    if field.tag() == "abbreviation" {
                        name.abbreviation = true;
                    } else {
                        name.plural = true;
                    }
                    names.push(name);
                } else if let Some(subfield) = field.field("name") {
                    if let Some(text) = subfield.text() {
                        let mut name = DefinitionName::parse(text);
                        if field.tag() == "abbreviation" {
                            name.abbreviation = true;
                        } else {
                            name.plural = true;
                        }
                        names.push(name);
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(name_attr) = item.attributes().get("name") {
        names.push(DefinitionName::parse(name_attr));
    }

    let mut retained = Vec::new();
    for name in names {
        if !name.name().is_empty()
            && !retained
                .iter()
                .any(|candidate: &DefinitionName| candidate.name() == name.name())
        {
            retained.push(name);
        }
    }
    retained
}
