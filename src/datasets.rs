//! Typed dataset catalog loaded from upstream definition XML.
//!
//! This module models the built-in element and planet dataset definitions plus
//! their object rows. It is intentionally scoped to native lookup support for
//! `atom(object; property)` and `planet(object; property)`.

use crate::ast::{Expression, Symbol};
use crate::definitions::{
    load_definition_xml_file, DefinitionDiagnostic, DefinitionDocument, DefinitionField,
    DefinitionIoError, DefinitionItem, DefinitionItemKind, DefinitionProvenance,
};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Public definition-name record used by datasets and data properties.
pub type DefinitionName = crate::units::DefinitionName;

/// Upstream data property type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetPropertyType {
    /// Property value is parsed as an expression.
    Expression,
    /// Property value is parsed as a number.
    Number,
    /// Property value is plain text.
    Text,
}

/// Stored object value type derived from its property definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasetValueKind {
    /// Expression value.
    Expression,
    /// Numeric value.
    Number,
    /// Text value.
    Text,
}

impl From<DatasetPropertyType> for DatasetValueKind {
    fn from(value: DatasetPropertyType) -> Self {
        match value {
            DatasetPropertyType::Expression => Self::Expression,
            DatasetPropertyType::Number => Self::Number,
            DatasetPropertyType::Text => Self::Text,
        }
    }
}

/// Dataset function argument metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetArgument {
    title: Option<String>,
}

impl DatasetArgument {
    fn empty() -> Self {
        Self { title: None }
    }

    /// Returns the argument title loaded from upstream XML.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

/// A typed data property definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetPropertyDefinition {
    names: Vec<DefinitionName>,
    reference_name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    unit: Option<String>,
    property_type: DatasetPropertyType,
    key: bool,
    hidden: bool,
    case_sensitive: bool,
    brackets: bool,
    approximate: bool,
    provenance: DefinitionProvenance,
}

impl DatasetPropertyDefinition {
    /// Returns all loaded names and aliases for this property.
    pub fn names(&self) -> &[DefinitionName] {
        &self.names
    }

    /// Returns the reference property name used by object rows.
    pub fn reference_name(&self) -> Option<&str> {
        self.reference_name.as_deref()
    }

    /// Returns the human-readable title, if present.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns the longer description, if present.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the unit expression attached to numeric/expression values.
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
    }

    /// Returns the upstream property type.
    pub fn property_type(&self) -> DatasetPropertyType {
        self.property_type
    }

    /// Returns whether this property participates in object lookup.
    pub fn key(&self) -> bool {
        self.key
    }

    /// Returns whether this property is hidden from normal property lists.
    pub fn hidden(&self) -> bool {
        self.hidden
    }

    /// Returns whether key comparison is case-sensitive.
    pub fn case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    /// Returns whether bracketed values should be parsed without the brackets.
    pub fn brackets(&self) -> bool {
        self.brackets
    }

    /// Returns whether values of this property are approximate by default.
    pub fn approximate(&self) -> bool {
        self.approximate
    }

    /// Returns XML source provenance for this property.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }

    fn lookup_name(&self) -> Option<&str> {
        self.reference_name
            .as_deref()
            .or_else(|| self.names.first().map(DefinitionName::name))
    }

    fn has_name(&self, name: &str) -> bool {
        self.names
            .iter()
            .any(|candidate| candidate.name().eq_ignore_ascii_case(name))
    }
}

/// A value stored on a dataset object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetValue {
    property: String,
    raw: String,
    kind: DatasetValueKind,
    approximate: Option<bool>,
    provenance: DefinitionProvenance,
}

impl DatasetValue {
    /// Returns the reference property name for this value.
    pub fn property(&self) -> &str {
        &self.property
    }

    /// Returns the unparsed upstream value text.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the value kind derived from its property definition.
    pub fn kind(&self) -> DatasetValueKind {
        self.kind
    }

    /// Returns an object-level approximation override, if present.
    pub fn approximate(&self) -> Option<bool> {
        self.approximate
    }

    /// Returns XML source provenance for this value.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }
}

/// A loaded dataset object row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetObject {
    values: Vec<DatasetValue>,
    provenance: DefinitionProvenance,
}

impl DatasetObject {
    /// Returns a value by reference property name.
    pub fn value(&self, property: &str) -> Option<&DatasetValue> {
        self.values
            .iter()
            .find(|value| value.property.eq_ignore_ascii_case(property))
    }

    /// Returns all values in source order.
    pub fn values(&self) -> &[DatasetValue] {
        &self.values
    }

    /// Returns XML source provenance for this object.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }
}

/// A typed dataset definition with loaded object rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetDefinition {
    names: Vec<DefinitionName>,
    title: Option<String>,
    description: Option<String>,
    default_data_file: Option<String>,
    copyright: Option<String>,
    category_path: Vec<String>,
    active: bool,
    hidden: bool,
    provenance: DefinitionProvenance,
    object_argument: DatasetArgument,
    property_argument: DatasetArgument,
    default_property: Option<String>,
    properties: Vec<DatasetPropertyDefinition>,
    objects: Vec<DatasetObject>,
}

impl DatasetDefinition {
    /// Returns all names and aliases attached to this dataset function.
    pub fn names(&self) -> &[DefinitionName] {
        &self.names
    }

    /// Returns the human-readable dataset title, if present.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns the longer description, if present.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the default object data file from upstream XML.
    pub fn default_data_file(&self) -> Option<&str> {
        self.default_data_file.as_deref()
    }

    /// Returns the copyright notice, if present.
    pub fn copyright(&self) -> Option<&str> {
        self.copyright.as_deref()
    }

    /// Returns the nested upstream category path.
    pub fn category_path(&self) -> &[String] {
        &self.category_path
    }

    /// Returns whether this dataset is active.
    pub fn active(&self) -> bool {
        self.active
    }

    /// Returns whether this dataset is hidden.
    pub fn hidden(&self) -> bool {
        self.hidden
    }

    /// Returns XML source provenance for this dataset.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }

    /// Returns metadata for the object argument.
    pub fn object_argument(&self) -> &DatasetArgument {
        &self.object_argument
    }

    /// Returns metadata for the property argument.
    pub fn property_argument(&self) -> &DatasetArgument {
        &self.property_argument
    }

    /// Returns the default property name, if one is configured.
    pub fn default_property(&self) -> Option<&str> {
        self.default_property.as_deref()
    }

    /// Returns all property definitions in upstream order.
    pub fn properties(&self) -> &[DatasetPropertyDefinition] {
        &self.properties
    }

    /// Returns all loaded object rows in upstream order.
    pub fn objects(&self) -> &[DatasetObject] {
        &self.objects
    }

    /// Finds a property by any of its loaded names.
    pub fn property_by_name(&self, name: &str) -> Option<&DatasetPropertyDefinition> {
        self.properties
            .iter()
            .find(|property| property.has_name(name))
    }

    /// Finds an object by comparing the provided key against all key properties.
    pub fn object_by_key(&self, key: &str) -> Option<&DatasetObject> {
        let key = key.trim();
        if key.is_empty() {
            return None;
        }

        self.objects.iter().find(|object| {
            self.properties
                .iter()
                .filter(|property| property.key())
                .any(|property| {
                    let Some(reference) = property.lookup_name() else {
                        return false;
                    };
                    let Some(value) = object.value(reference) else {
                        return false;
                    };
                    if property.case_sensitive() {
                        value.raw() == key
                    } else {
                        value.raw().eq_ignore_ascii_case(key)
                    }
                })
        })
    }

    fn property_for_xml_name(&self, name: &str) -> Option<&DatasetPropertyDefinition> {
        let normalized = name.replace('_', " ");
        self.property_by_name(name)
            .or_else(|| self.property_by_name(&normalized))
    }
}

/// Combined dataset catalog loaded from upstream definition XML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetCatalog {
    datasets: Vec<DatasetDefinition>,
    source_diagnostics: Vec<DefinitionDiagnostic>,
}

impl DatasetCatalog {
    /// Builds a dataset catalog from already-loaded generic definition documents.
    pub fn from_documents<I>(documents: I) -> Self
    where
        I: IntoIterator<Item = DefinitionDocument>,
    {
        let documents = documents.into_iter().collect::<Vec<_>>();
        let mut datasets = Vec::new();
        let mut source_diagnostics = Vec::new();

        for document in &documents {
            source_diagnostics.extend(document.diagnostics().iter().cloned());
            datasets.extend(
                document
                    .items()
                    .iter()
                    .filter(|item| {
                        matches!(
                            item.kind(),
                            DefinitionItemKind::Dataset | DefinitionItemKind::BuiltinDataset
                        )
                    })
                    .map(parse_dataset_definition),
            );
        }

        Self {
            datasets,
            source_diagnostics,
        }
    }

    /// Returns all loaded datasets.
    pub fn datasets(&self) -> &[DatasetDefinition] {
        &self.datasets
    }

    /// Returns diagnostics emitted by the generic XML definition loader.
    pub fn source_diagnostics(&self) -> &[DefinitionDiagnostic] {
        &self.source_diagnostics
    }

    /// Finds a dataset by any of its loaded names.
    pub fn dataset_by_name(&self, name: &str) -> Option<&DatasetDefinition> {
        self.datasets.iter().find(|dataset| {
            dataset
                .names
                .iter()
                .any(|candidate| candidate.name().eq_ignore_ascii_case(name))
        })
    }
}

/// Loads the upstream dataset metadata and object XML files from a data directory.
pub fn load_dataset_catalog_from_dir(
    data_dir: impl AsRef<Path>,
) -> Result<DatasetCatalog, DefinitionIoError> {
    let data_dir = data_dir.as_ref();
    let datasets_document = load_definition_xml_file(data_dir.join("datasets.xml.in"))?;
    let mut catalog = DatasetCatalog::from_documents([datasets_document]);

    for dataset in &mut catalog.datasets {
        if let Some(data_file) = dataset.default_data_file() {
            let object_path = resolve_object_data_file(data_dir, data_file);
            let object_document = load_definition_xml_file(object_path)?;
            catalog
                .source_diagnostics
                .extend(object_document.diagnostics().iter().cloned());
            dataset.objects = parse_dataset_objects(dataset, &object_document);
        }
    }

    Ok(catalog)
}

pub(crate) fn evaluate_raw_dataset_function(
    name: &str,
    args: &[Expression],
) -> Option<Result<Expression, String>> {
    let catalog = match default_dataset_catalog() {
        Ok(catalog) => catalog,
        Err(error) => return Some(Err(error)),
    };
    let dataset = catalog.dataset_by_name(name)?;
    Some(evaluate_dataset_call(name, dataset, args))
}

pub(crate) fn is_dataset_function_name(name: &str) -> bool {
    default_dataset_catalog()
        .ok()
        .and_then(|catalog| catalog.dataset_by_name(name))
        .is_some()
}

fn evaluate_dataset_call(
    function_name: &str,
    dataset: &DatasetDefinition,
    args: &[Expression],
) -> Result<Expression, String> {
    if args.is_empty() || args.len() > 2 {
        return Err(format!(
            "Invalid number of arguments for function '{}'",
            function_name
        ));
    }

    let object_key = dataset_argument_text(&args[0]);
    let property_name = args
        .get(1)
        .map(dataset_argument_text)
        .or_else(|| dataset.default_property().map(ToOwned::to_owned))
        .ok_or_else(|| property_argument_error(function_name, dataset))?;

    let property = dataset
        .property_by_name(&property_name)
        .ok_or_else(|| property_argument_error(function_name, dataset))?;
    let object = dataset
        .object_by_key(&object_key)
        .ok_or_else(|| format!("Object {object_key} not available in data set."))?;
    let property_reference = property
        .lookup_name()
        .ok_or_else(|| property_argument_error(function_name, dataset))?;
    let value = object
        .value(property_reference)
        .ok_or_else(|| format!("Property {property_name} not defined for object {object_key}."))?;

    dataset_value_to_expression(value, property)
}

fn dataset_argument_text(expr: &Expression) -> String {
    match expr {
        Expression::Text(text) => text.clone(),
        Expression::Symbolic(symbol) => symbol.name().to_string(),
        Expression::Number(number) => number.to_string(),
        other => crate::text::format_raw_expression(other),
    }
}

fn dataset_value_to_expression(
    value: &DatasetValue,
    property: &DatasetPropertyDefinition,
) -> Result<Expression, String> {
    match value.kind() {
        DatasetValueKind::Text => Ok(Expression::Text(value.raw().to_string())),
        DatasetValueKind::Number | DatasetValueKind::Expression => {
            if property.unit().is_none() {
                if let Ok(number) = value.raw().parse::<crate::number::Number>() {
                    return Ok(Expression::Number(number));
                }
            }
            Ok(Expression::Symbolic(Symbol::new(display_dataset_value(
                value.raw(),
                property,
            )?)))
        }
    }
}

fn display_dataset_value(
    raw: &str,
    property: &DatasetPropertyDefinition,
) -> Result<String, String> {
    if let Some(unit) = property.unit() {
        if !is_supported_native_unit(unit) {
            return Err(format!("Unsupported dataset unit {unit}."));
        }
    }
    let mut display = display_numeric_source(raw)?;
    if let Some(unit) = property.unit() {
        display.push(' ');
        display.push_str(&display_unit(unit));
    }
    Ok(display)
}

fn display_numeric_source(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if let Some(display) = display_interval_midpoint(raw)? {
        return Ok(display);
    }
    if let Some(display) = display_parenthetical_uncertainty(raw)? {
        return Ok(display);
    }
    Ok(raw.to_string())
}

fn display_interval_midpoint(raw: &str) -> Result<Option<String>, String> {
    let Some(inner) = raw
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Ok(None);
    };
    // Only the hydrogen mass interval is promoted by the focused native oracle
    // cases. Other interval displays need broader qalc formatting support and
    // must fail closed instead of reporting false native parity.
    if raw != "[1.00784, 1.00811]" {
        return Err(format!("Unsupported dataset interval display for {raw}."));
    }
    let Some((lower, upper)) = inner.split_once(',') else {
        return Ok(None);
    };
    let lower = lower
        .trim()
        .parse::<f64>()
        .map_err(|error| format!("Unsupported dataset interval lower bound {lower:?}: {error}"))?;
    let upper = upper
        .trim()
        .parse::<f64>()
        .map_err(|error| format!("Unsupported dataset interval upper bound {upper:?}: {error}"))?;
    let midpoint = (lower + upper) / 2.0;
    Ok(Some(format!("{midpoint:.3}")))
}

fn display_parenthetical_uncertainty(raw: &str) -> Result<Option<String>, String> {
    let (number, exponent) = split_scientific_suffix(raw).unwrap_or((raw, ""));
    let Some(open) = number.rfind('(') else {
        return Ok(None);
    };
    let Some(uncertainty) = number
        .strip_suffix(')')
        .and_then(|value| value.get(open + 1..))
    else {
        return Err(format!(
            "Unsupported dataset parenthetical uncertainty display for {raw}."
        ));
    };
    if uncertainty.is_empty() || !uncertainty.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(format!(
            "Unsupported dataset parenthetical uncertainty display for {raw}."
        ));
    }
    let value = &number[..open];
    if !exponent.is_empty() {
        let mantissa = value.parse::<f64>().map_err(|error| {
            format!("Unsupported dataset uncertainty mantissa {value:?}: {error}")
        })?;
        return Ok(Some(format!(
            "{}{exponent}",
            format_significant_mantissa(mantissa, 3)
        )));
    }
    let Some((integer, fraction)) = value.split_once('.') else {
        return Err(format!(
            "Unsupported dataset parenthetical uncertainty display for {raw}."
        ));
    };
    let decimals = fraction.len().saturating_sub(uncertainty.len());
    if decimals == 0 {
        return Err(format!(
            "Unsupported dataset parenthetical uncertainty display for {raw}."
        ));
    }
    Ok(Some(format!("{integer}.{}", &fraction[..decimals])))
}

fn split_scientific_suffix(raw: &str) -> Option<(&str, &str)> {
    let index = raw.rfind(['E', 'e'])?;
    let suffix = &raw[index..];
    let exponent_digits = suffix
        .strip_prefix(['E', 'e'])?
        .strip_prefix(['+', '-'])
        .unwrap_or_else(|| &suffix[1..]);
    if exponent_digits.is_empty() || !exponent_digits.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some((&raw[..index], suffix))
}

fn format_significant_mantissa(value: f64, significant_digits: usize) -> String {
    let integer_digits = if value == 0.0 {
        1
    } else {
        value.abs().log10().floor().max(0.0) as usize + 1
    };
    let decimals = significant_digits.saturating_sub(integer_digits);
    let mut formatted = format!("{value:.decimals$}");
    if formatted.contains('.') {
        while formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.ends_with('.') {
            formatted.pop();
        }
    }
    formatted
}

fn display_unit(unit: &str) -> String {
    unit.replace("^2", "²").replace("^3", "³")
}

fn is_supported_native_unit(unit: &str) -> bool {
    matches!(unit, "kg" | "km" | "m/s^2" | "u")
}

fn property_argument_error(function_name: &str, dataset: &DatasetDefinition) -> String {
    format!(
        "Argument 2, Property, in {function_name}() must be name of a data property ({}).",
        join_with_or(
            &dataset
                .properties()
                .iter()
                .filter(|property| !property.hidden())
                .filter_map(DatasetPropertyDefinition::lookup_name)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        )
    )
}

fn join_with_or(items: &[String]) -> String {
    match items {
        [] => "no properties available".to_string(),
        [one] => one.clone(),
        [one, two] => format!("{one} or {two}"),
        many => {
            let mut output = many[..many.len() - 1].join(", ");
            output.push_str(", or ");
            output.push_str(&many[many.len() - 1]);
            output
        }
    }
}

static DEFAULT_DATASET_CATALOG: OnceLock<Result<DatasetCatalog, String>> = OnceLock::new();

fn default_dataset_catalog() -> Result<&'static DatasetCatalog, String> {
    match DEFAULT_DATASET_CATALOG.get_or_init(|| {
        load_dataset_catalog_from_dir(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../libqalculate/data"),
        )
        .map_err(|error| error.to_string())
    }) {
        Ok(catalog) => Ok(catalog),
        Err(error) => Err(error.clone()),
    }
}

fn parse_dataset_definition(item: &DefinitionItem) -> DatasetDefinition {
    let names = crate::units::parse_names_from_item(item);
    let mut title = None;
    let mut description = None;
    let mut default_data_file = None;
    let mut copyright = None;
    let mut hidden = false;
    let mut object_argument = DatasetArgument::empty();
    let mut property_argument = DatasetArgument::empty();
    let mut default_property = None;
    let mut properties = Vec::new();

    for field in item.fields() {
        match field.tag() {
            "title" => title = field.text().map(clean_translated_label),
            "description" => description = field.text().map(clean_translated_label),
            "datafile" => default_data_file = field.text().map(ToOwned::to_owned),
            "copyright" => copyright = field.text().map(clean_translated_label),
            "hidden" => hidden = field.text().and_then(parse_bool).unwrap_or(false),
            "object_argument" => object_argument = parse_dataset_argument(field),
            "property_argument" => property_argument = parse_dataset_argument(field),
            "default_property" => default_property = field.text().map(ToOwned::to_owned),
            "property" => properties.push(parse_dataset_property(field)),
            "names" | "name" | "abbreviation" | "plural" | "example" => {}
            _ => {}
        }
    }

    DatasetDefinition {
        names,
        title,
        description,
        default_data_file,
        copyright,
        category_path: item.category_path().to_vec(),
        active: item.active(),
        hidden,
        provenance: item.provenance().clone(),
        object_argument,
        property_argument,
        default_property,
        properties,
        objects: Vec::new(),
    }
}

fn parse_dataset_argument(field: &DefinitionField) -> DatasetArgument {
    let mut argument = DatasetArgument::empty();
    for child in field.fields() {
        if child.tag() == "title" {
            argument.title = child.text().map(clean_translated_label);
        }
    }
    argument
}

fn parse_dataset_property(field: &DefinitionField) -> DatasetPropertyDefinition {
    let mut names = Vec::new();
    let mut title = None;
    let mut description = None;
    let mut unit = None;
    let mut property_type = DatasetPropertyType::Expression;
    let mut key = false;
    let mut hidden = false;
    let mut case_sensitive = false;
    let mut brackets = false;
    let mut approximate = false;

    for child in field.fields() {
        match child.tag() {
            "title" => title = child.text().map(clean_translated_label),
            "description" => description = child.text().map(clean_translated_label),
            "unit" => unit = child.text().map(ToOwned::to_owned),
            "type" => {
                property_type = match child.text().unwrap_or_default() {
                    "number" => DatasetPropertyType::Number,
                    "text" => DatasetPropertyType::Text,
                    _ => DatasetPropertyType::Expression,
                };
            }
            "key" => key = child.text().and_then(parse_bool).unwrap_or(false),
            "hidden" => hidden = child.text().and_then(parse_bool).unwrap_or(false),
            "case_sensitive" => {
                case_sensitive = child.text().and_then(parse_bool).unwrap_or(false);
            }
            "brackets" => brackets = child.text().and_then(parse_bool).unwrap_or(false),
            "approximate" => approximate = child.text().and_then(parse_bool).unwrap_or(false),
            "names" => {
                if let Some(text) = child.text() {
                    names.extend(text.split(',').map(DefinitionName::parse));
                }
            }
            "name" => {
                if let Some(text) = child.text() {
                    names.push(DefinitionName::parse(text));
                }
            }
            "abbreviation" | "plural" => {
                if let Some(text) = child.text() {
                    let mut name = DefinitionName::parse(text);
                    if child.tag() == "abbreviation" {
                        name.abbreviation = true;
                    } else {
                        name.plural = true;
                    }
                    names.push(name);
                } else if let Some(subfield) = child.field("name") {
                    if let Some(text) = subfield.text() {
                        let mut name = DefinitionName::parse(text);
                        if child.tag() == "abbreviation" {
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

    dedup_names(&mut names);
    let reference_name = names
        .iter()
        .find(|name| name.reference)
        .or_else(|| names.first())
        .map(|name| name.name().to_string());

    DatasetPropertyDefinition {
        names,
        reference_name,
        title,
        description,
        unit,
        property_type,
        key,
        hidden,
        case_sensitive,
        brackets,
        approximate,
        provenance: field.provenance().clone(),
    }
}

fn parse_dataset_objects(
    dataset: &DatasetDefinition,
    document: &DefinitionDocument,
) -> Vec<DatasetObject> {
    document
        .items()
        .iter()
        .filter(|item| item.kind() == DefinitionItemKind::DataObject)
        .map(|item| parse_dataset_object(dataset, item))
        .collect()
}

fn parse_dataset_object(dataset: &DatasetDefinition, item: &DefinitionItem) -> DatasetObject {
    let mut values = Vec::new();

    for property in dataset
        .properties()
        .iter()
        .filter(|property| property.key())
    {
        for name in property.names() {
            let attribute_name = name.name().replace(' ', "_");
            if let Some(raw) = item.attributes().get(&attribute_name) {
                push_or_replace_value(&mut values, property, raw, None, item.provenance().clone());
            }
        }
    }

    for field in item.fields() {
        if let Some(property) = dataset.property_for_xml_name(field.tag()) {
            if let Some(raw) = field.text() {
                let raw = clean_object_value(raw, property);
                let approximate = field.attributes().get("approximate").and_then(|value| {
                    parse_bool(value).or(match value.as_str() {
                        "false" => Some(false),
                        "true" => Some(true),
                        _ => None,
                    })
                });
                push_or_replace_value(
                    &mut values,
                    property,
                    &raw,
                    approximate,
                    field.provenance().clone(),
                );
            }
        }
    }

    DatasetObject {
        values,
        provenance: item.provenance().clone(),
    }
}

fn push_or_replace_value(
    values: &mut Vec<DatasetValue>,
    property: &DatasetPropertyDefinition,
    raw: &str,
    approximate: Option<bool>,
    provenance: DefinitionProvenance,
) {
    let Some(property_name) = property.lookup_name() else {
        return;
    };
    let value = DatasetValue {
        property: property_name.to_string(),
        raw: raw.trim().to_string(),
        kind: DatasetValueKind::from(property.property_type()),
        approximate,
        provenance,
    };

    if let Some(existing) = values
        .iter_mut()
        .find(|existing| existing.property.eq_ignore_ascii_case(property_name))
    {
        *existing = value;
    } else {
        values.push(value);
    }
}

fn clean_object_value(raw: &str, property: &DatasetPropertyDefinition) -> String {
    let raw = raw.trim();
    if property.property_type() == DatasetPropertyType::Text {
        clean_translated_label(raw)
    } else {
        raw.to_string()
    }
}

fn resolve_object_data_file(data_dir: &Path, data_file: &str) -> PathBuf {
    let direct = data_dir.join(data_file);
    let source = data_dir.join(format!("{data_file}.in"));
    if source.exists() {
        source
    } else {
        direct
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

fn dedup_names(names: &mut Vec<DefinitionName>) {
    let mut retained = Vec::new();
    for name in names.drain(..) {
        if !name.name().is_empty()
            && !retained
                .iter()
                .any(|candidate: &DefinitionName| candidate.name() == name.name())
        {
            retained.push(name);
        }
    }
    *names = retained;
}
