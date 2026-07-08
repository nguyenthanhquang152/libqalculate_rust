//! Typed prefix and unit definition catalogs loaded from definitions.
//!
//! This module provides typed representations of prefixes, units, and currency definitions,
//! parsed from the generic XML definition documents. It preserves flags, active state,
//! category paths, metadata, relations, and source provenance.

use crate::definitions::{
    load_definition_xml_file, load_definition_xml_str, DefinitionDocument, DefinitionIoError,
    DefinitionItem, DefinitionItemKind, DefinitionProvenance,
};
use crate::parser::names::StaticRegistry;
use std::collections::HashSet;
use std::path::Path;

/// Public prefix kind name used by the typed prefix/unit loader API.
pub type PrefixKind = PrefixType;

/// Public unit kind name used by the typed prefix/unit loader API.
pub type UnitKind = UnitType;

/// Public definition-name record used by prefixes and units.
pub type DefinitionName = StructuredName;

/// Category of prefix (SI decimal or binary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixType {
    /// Base-10 prefix (e.g. kilo, micro).
    Decimal,
    /// Base-2 prefix (e.g. kibi, mebi).
    Binary,
}

/// Category of unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitType {
    /// Base unit (e.g. meter).
    Base,
    /// Alias unit (e.g. micron).
    Alias,
    /// Composite unit (e.g. kilometer).
    Composite,
    /// Built-in unit (e.g. currency).
    Builtin,
}

/// A structured name with parsed flags from the XML names declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredName {
    /// The normalized clean name string.
    pub name: String,
    /// Raw upstream flag string before `:`, if present.
    pub flags: String,
    /// Whether the name is marked as an abbreviation.
    pub abbreviation: bool,
    /// Whether the name is case sensitive.
    pub case_sensitive: bool,
    /// Whether the name contains Unicode characters.
    pub unicode: bool,
    /// Whether the name represents a plural form.
    pub plural: bool,
    /// Whether the name is a fixed reference.
    pub reference: bool,
    /// Whether the name should be avoided for user input.
    pub avoid_input: bool,
    /// Whether the name is only for completion.
    pub completion_only: bool,
    /// Whether the name has a suffix.
    pub suffix: bool,
}

impl StructuredName {
    /// Parses a single name entry with its flags.
    pub fn parse(raw: &str) -> Self {
        let raw = raw.trim();
        let mut clean_raw = raw;
        if clean_raw.starts_with('!') {
            if let Some(second_bang) = clean_raw[1..].find('!') {
                clean_raw = &clean_raw[second_bang + 2..];
            }
        }
        clean_raw = clean_raw.trim();

        let mut abbreviation = false;
        let mut case_sensitive = false;
        let mut unicode = false;
        let mut plural = false;
        let mut reference = false;
        let mut avoid_input = false;
        let mut completion_only = false;
        let mut suffix = false;

        let name_str;
        let mut flags = String::new();
        if let Some((flags_str, rest)) = clean_raw.split_once(':') {
            flags = flags_str.to_string();
            name_str = rest.trim();
            let mut b = true;
            for c in flags_str.chars() {
                match c {
                    '-' => b = false,
                    'a' => {
                        abbreviation = b;
                        b = true;
                    }
                    'c' => {
                        case_sensitive = b;
                        b = true;
                    }
                    'i' => {
                        avoid_input = b;
                        b = true;
                    }
                    'p' => {
                        plural = b;
                        b = true;
                    }
                    'r' => {
                        reference = b;
                        b = true;
                    }
                    's' => {
                        suffix = b;
                        b = true;
                    }
                    'u' => {
                        unicode = b;
                        b = true;
                    }
                    'o' => {
                        completion_only = b;
                        b = true;
                    }
                    _ => {}
                }
            }
        } else {
            name_str = clean_raw;
            if name_str.chars().count() == 1 {
                abbreviation = true;
                case_sensitive = true;
            }
        }

        let name_str = name_str.trim_start_matches('-').trim();

        Self {
            name: name_str.to_string(),
            flags,
            abbreviation,
            case_sensitive,
            unicode,
            plural,
            reference,
            avoid_input,
            completion_only,
            suffix,
        }
    }

    /// Returns the parsed name text without upstream flags.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the raw upstream flags attached to this name.
    pub fn flags(&self) -> &str {
        &self.flags
    }
}

/// Machine-readable warning category emitted while building typed unit data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildDiagnosticKind {
    /// A prefix XML child field is preserved by the generic loader but not modeled here.
    UnsupportedPrefixField,
    /// A unit XML child field is preserved by the generic loader but not modeled here.
    UnsupportedUnitField,
}

/// Structured warning emitted while building typed prefix/unit data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildDiagnostic {
    kind: BuildDiagnosticKind,
    message: String,
    provenance: DefinitionProvenance,
    tag: Option<String>,
    name: Option<String>,
}

impl BuildDiagnostic {
    fn unsupported_prefix_field(
        message: String,
        provenance: &DefinitionProvenance,
        tag: Option<String>,
        name: Option<String>,
    ) -> Self {
        Self {
            kind: BuildDiagnosticKind::UnsupportedPrefixField,
            message,
            provenance: provenance.clone(),
            tag,
            name,
        }
    }

    fn unsupported_unit_field(
        message: String,
        provenance: &DefinitionProvenance,
        tag: Option<String>,
        name: Option<String>,
    ) -> Self {
        Self {
            kind: BuildDiagnosticKind::UnsupportedUnitField,
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

/// Extracts structured names from a generic definition item.
pub fn parse_names_from_item(item: &DefinitionItem) -> Vec<StructuredName> {
    let mut names = Vec::new();
    if let Some(name_attr) = item.attributes().get("name") {
        names.push(StructuredName::parse(name_attr));
    }
    for field in item.fields() {
        match field.tag() {
            "names" => {
                if let Some(text) = field.text() {
                    for part in text.split(',') {
                        names.push(StructuredName::parse(part));
                    }
                }
            }
            "name" => {
                if let Some(text) = field.text() {
                    names.push(StructuredName::parse(text));
                }
            }
            "abbreviation" | "plural" => {
                if let Some(text) = field.text() {
                    let mut name = StructuredName::parse(text);
                    if field.tag() == "abbreviation" {
                        name.abbreviation = true;
                    } else {
                        name.plural = true;
                    }
                    names.push(name);
                } else if let Some(subfield) = field.field("name") {
                    if let Some(text) = subfield.text() {
                        let mut name = StructuredName::parse(text);
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
    let mut seen = HashSet::new();
    names.retain(|n| !n.name.is_empty() && seen.insert(n.name.clone()));
    names
}

/// A parsed prefix definition.
#[derive(Debug, Clone)]
pub struct PrefixDefinition {
    /// Decimal or binary prefix category.
    pub kind: PrefixType,
    /// Exponent value (SI base-10 exponent, or base-2 power-of-10 index).
    pub exponent: i32,
    /// All names/aliases mapped for this prefix.
    pub names: Vec<StructuredName>,
    /// Active state flag.
    pub active: bool,
    /// Upstream XML source location provenance.
    pub provenance: DefinitionProvenance,
}

/// Catalog of loaded prefix definitions.
#[derive(Debug, Clone)]
pub struct PrefixCatalog {
    /// All parsed prefix definitions.
    pub prefixes: Vec<PrefixDefinition>,
}

impl PrefixCatalog {
    /// Loads a prefix catalog from a generic document.
    pub fn from_document(doc: &DefinitionDocument) -> Result<Self, String> {
        let mut diagnostics = Vec::new();
        let cat = Self::from_document_with_diagnostics(doc, &mut diagnostics);
        Ok(cat)
    }

    /// Loads a prefix catalog while tracking builder warnings.
    pub fn from_document_with_diagnostics(
        doc: &DefinitionDocument,
        diagnostics: &mut Vec<BuildDiagnostic>,
    ) -> Self {
        let mut prefixes = Vec::new();

        for item in doc
            .items()
            .iter()
            .filter(|i| i.kind() == DefinitionItemKind::Prefix)
        {
            let kind = match item.attributes().get("type").map(String::as_str) {
                Some("binary") => PrefixType::Binary,
                _ => PrefixType::Decimal,
            };

            let mut exponent = 0;
            let names = parse_names_from_item(item);

            for field in item.fields() {
                match field.tag() {
                    "exponent" => {
                        if let Some(text) = field.text() {
                            exponent = text.parse().unwrap_or(0);
                        }
                    }
                    "names" | "name" | "abbreviation" | "plural" => {}
                    other => {
                        diagnostics.push(BuildDiagnostic::unsupported_prefix_field(
                            format!("Unsupported prefix field <{other}>"),
                            field.provenance(),
                            Some(other.to_string()),
                            None,
                        ));
                    }
                }
            }

            prefixes.push(PrefixDefinition {
                kind,
                exponent,
                names,
                active: item.active(),
                provenance: item.provenance().clone(),
            });
        }

        Self { prefixes }
    }

    /// Finds a prefix definition by name.
    pub fn find_by_name(&self, name: &str) -> Option<&PrefixDefinition> {
        self.prefixes
            .iter()
            .find(|p| p.names.iter().any(|n| n.name == name))
    }

    /// Registers all active prefix names into the static name registry.
    pub fn register_all(&self, registry: &mut StaticRegistry) {
        for prefix in &self.prefixes {
            if prefix.active {
                for name_rec in &prefix.names {
                    registry.add_prefix(&name_rec.name);
                }
            }
        }
    }
}

impl PrefixDefinition {
    /// Returns the decimal or binary prefix category.
    pub fn kind(&self) -> PrefixKind {
        self.kind
    }

    /// Returns the upstream exponent value for this prefix.
    pub fn exponent(&self) -> i32 {
        self.exponent
    }

    /// Returns all names and aliases attached to this prefix.
    pub fn names(&self) -> &[DefinitionName] {
        &self.names
    }

    /// Returns whether this prefix is active.
    pub fn active(&self) -> bool {
        self.active
    }

    /// Returns XML source provenance for this prefix.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }
}

/// Composite unit part definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitPart {
    /// Unit name.
    pub unit: String,
    /// Prefix (as exponent offset or prefix symbol).
    pub prefix: Option<String>,
    /// Exponent.
    pub exponent: Option<String>,
}

/// Base relation for unit aliases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitBase {
    /// Base unit name.
    pub unit: String,
    /// Relation scale factor string.
    pub relation: Option<String>,
    /// Exponent.
    pub exponent: Option<String>,
    /// Mix.
    pub mix: Option<String>,
}

/// A parsed unit definition.
#[derive(Debug, Clone)]
pub struct UnitDefinition {
    /// Base, alias, composite, or builtin unit.
    pub kind: UnitType,
    /// All names/aliases mapped for this unit.
    pub names: Vec<StructuredName>,
    /// Long human-readable description title.
    pub title: Option<String>,
    /// Longer human-readable description text, when upstream provides one.
    pub description: Option<String>,
    /// Unit system label, such as `SI`, when present.
    pub system: Option<String>,
    /// Country metadata attached to currency definitions.
    pub countries: Vec<String>,
    /// Subcategory folder path.
    pub category_path: Vec<String>,
    /// Active state flag.
    pub active: bool,
    /// Hidden state flag.
    pub hidden: bool,
    /// Upstream XML source location provenance.
    pub provenance: DefinitionProvenance,
    /// Whether prefixes are allowed to attach.
    pub use_with_prefixes: bool,
    /// Minimum allowed prefix exponent, when upstream specifies one.
    pub prefix_min: Option<i32>,
    /// Maximum allowed prefix exponent, when upstream specifies one.
    pub prefix_max: Option<i32>,
    /// Default prefix exponent, when upstream specifies one.
    pub prefix_default: Option<i32>,
    /// Composite part definitions (for Composite unit).
    pub parts: Vec<UnitPart>,
    /// Base unit relation definitions (for Alias unit).
    pub bases: Vec<UnitBase>,
}

/// Catalog of loaded unit definitions.
#[derive(Debug, Clone)]
pub struct UnitCatalog {
    /// All parsed unit definitions.
    pub units: Vec<UnitDefinition>,
}

impl UnitCatalog {
    /// Loads a unit catalog from a generic document.
    pub fn from_document(doc: &DefinitionDocument) -> Result<Self, String> {
        let mut diagnostics = Vec::new();
        let cat = Self::from_document_with_diagnostics(doc, &mut diagnostics);
        Ok(cat)
    }

    /// Loads a unit catalog while tracking builder warnings.
    pub fn from_document_with_diagnostics(
        doc: &DefinitionDocument,
        diagnostics: &mut Vec<BuildDiagnostic>,
    ) -> Self {
        let mut units = Vec::new();

        for item in doc.items().iter().filter(|i| {
            i.kind() == DefinitionItemKind::Unit || i.kind() == DefinitionItemKind::BuiltinUnit
        }) {
            let kind = match item.attributes().get("type").map(String::as_str) {
                Some("composite") => UnitType::Composite,
                Some("alias") => UnitType::Alias,
                Some("base") => UnitType::Base,
                _ => {
                    if item.kind() == DefinitionItemKind::BuiltinUnit {
                        UnitType::Builtin
                    } else {
                        UnitType::Base
                    }
                }
            };

            let names = parse_names_from_item(item);
            let mut title = None;
            let mut description = None;
            let mut system = None;
            let mut countries = Vec::new();
            let mut hidden = false;
            let mut use_with_prefixes = false;
            let mut prefix_min = None;
            let mut prefix_max = None;
            let mut prefix_default = None;
            let mut parts = Vec::new();
            let mut bases = Vec::new();

            for field in item.fields() {
                match field.tag() {
                    "title" => title = field.text().map(clean_translated_label),
                    "description" => description = field.text().map(clean_translated_label),
                    "system" => system = field.text().map(String::from),
                    "countries" => {
                        if let Some(text) = field.text() {
                            countries.extend(parse_comma_separated_metadata(text));
                        }
                    }
                    "hidden" => hidden = field.text() == Some("true"),
                    "use_with_prefixes" => {
                        use_with_prefixes = field.text() == Some("true");
                        prefix_min = parse_i32_attribute(field, "min");
                        prefix_max = parse_i32_attribute(field, "max");
                        prefix_default = parse_i32_attribute(field, "default");
                    }
                    "part" => {
                        let unit = field
                            .field("unit")
                            .and_then(|f| f.text())
                            .unwrap_or_default()
                            .to_string();
                        let prefix = field
                            .field("prefix")
                            .and_then(|f| f.text())
                            .map(String::from);
                        let exponent = field
                            .field("exponent")
                            .and_then(|f| f.text())
                            .map(String::from);
                        parts.push(UnitPart {
                            unit,
                            prefix,
                            exponent,
                        });
                    }
                    "base" => {
                        let unit = field
                            .field("unit")
                            .and_then(|f| f.text())
                            .unwrap_or_default()
                            .to_string();
                        let relation = field
                            .field("relation")
                            .and_then(|f| f.text())
                            .map(String::from);
                        let exponent = field
                            .field("exponent")
                            .and_then(|f| f.text())
                            .map(String::from);
                        let mix = field.field("mix").and_then(|f| f.text()).map(String::from);
                        bases.push(UnitBase {
                            unit,
                            relation,
                            exponent,
                            mix,
                        });
                    }
                    "names" | "name" | "abbreviation" | "plural" => {}
                    other => {
                        diagnostics.push(BuildDiagnostic::unsupported_unit_field(
                            format!("Unsupported unit field <{other}>"),
                            field.provenance(),
                            Some(other.to_string()),
                            None,
                        ));
                    }
                }
            }

            units.push(UnitDefinition {
                kind,
                names,
                title,
                description,
                system,
                countries,
                category_path: item.category_path().to_vec(),
                active: item.active(),
                hidden,
                provenance: item.provenance().clone(),
                use_with_prefixes,
                prefix_min,
                prefix_max,
                prefix_default,
                parts,
                bases,
            });
        }

        Self { units }
    }

    /// Finds a unit definition by name.
    pub fn find_by_name(&self, name: &str) -> Option<&UnitDefinition> {
        self.units
            .iter()
            .find(|u| u.names.iter().any(|n| n.name == name))
    }

    /// Registers all active unit names into the static name registry.
    pub fn register_all(&self, registry: &mut StaticRegistry) {
        for unit in &self.units {
            if unit.active {
                for name_rec in &unit.names {
                    registry.add_unit(&name_rec.name);
                }
            }
        }
    }
}

impl UnitPart {
    /// Returns the unit name referenced by this composite part.
    pub fn unit(&self) -> &str {
        &self.unit
    }

    /// Returns the raw prefix field attached to this part, if present.
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    /// Returns the prefix exponent attached to this part, if it parses as an integer.
    pub fn prefix_exponent(&self) -> Option<i32> {
        self.prefix.as_deref().and_then(|value| value.parse().ok())
    }

    /// Returns the raw exponent field attached to this part, if present.
    pub fn exponent_text(&self) -> Option<&str> {
        self.exponent.as_deref()
    }

    /// Returns the exponent attached to this part, defaulting to one when omitted.
    pub fn exponent(&self) -> i32 {
        self.exponent
            .as_deref()
            .and_then(|value| value.parse().ok())
            .unwrap_or(1)
    }
}

impl UnitBase {
    /// Returns the base unit name referenced by this alias.
    pub fn unit(&self) -> &str {
        &self.unit
    }

    /// Returns the raw relation scale expression, if present.
    pub fn relation(&self) -> Option<&str> {
        self.relation.as_deref()
    }

    /// Returns the raw exponent field attached to this relation, if present.
    pub fn exponent_text(&self) -> Option<&str> {
        self.exponent.as_deref()
    }

    /// Returns the exponent attached to this relation, defaulting to one when omitted.
    pub fn exponent(&self) -> i32 {
        self.exponent
            .as_deref()
            .and_then(|value| value.parse().ok())
            .unwrap_or(1)
    }

    /// Returns the raw mix field attached to this relation, if present.
    pub fn mix(&self) -> Option<&str> {
        self.mix.as_deref()
    }
}

impl UnitDefinition {
    /// Returns the parsed unit category.
    pub fn kind(&self) -> UnitKind {
        self.kind
    }

    /// Returns all names and aliases attached to this unit.
    pub fn names(&self) -> &[DefinitionName] {
        &self.names
    }

    /// Returns the human-readable unit title, if present.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns the longer human-readable unit description, if present.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the unit system label, if present.
    pub fn system(&self) -> Option<&str> {
        self.system.as_deref()
    }

    /// Returns the country metadata attached to this unit, if any.
    pub fn countries(&self) -> &[String] {
        &self.countries
    }

    /// Returns the nested upstream category path for this unit.
    pub fn category_path(&self) -> &[String] {
        &self.category_path
    }

    /// Returns whether this unit is active.
    pub fn active(&self) -> bool {
        self.active
    }

    /// Returns whether this unit is hidden upstream.
    pub fn hidden(&self) -> bool {
        self.hidden
    }

    /// Returns XML source provenance for this unit.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }

    /// Returns whether prefixes are allowed to attach to this unit.
    pub fn use_with_prefixes(&self) -> bool {
        self.use_with_prefixes
    }

    /// Returns the minimum allowed prefix exponent, if specified upstream.
    pub fn prefix_min(&self) -> Option<i32> {
        self.prefix_min
    }

    /// Returns the maximum allowed prefix exponent, if specified upstream.
    pub fn prefix_max(&self) -> Option<i32> {
        self.prefix_max
    }

    /// Returns the default prefix exponent, if specified upstream.
    pub fn prefix_default(&self) -> Option<i32> {
        self.prefix_default
    }

    /// Returns composite part definitions.
    pub fn parts(&self) -> &[UnitPart] {
        &self.parts
    }

    /// Returns the first base relation definition for alias units, if present.
    pub fn base(&self) -> Option<&UnitBase> {
        self.bases.first()
    }

    /// Returns all base relation definitions.
    pub fn bases(&self) -> &[UnitBase] {
        &self.bases
    }
}

/// Combined prefix and unit catalog loaded from upstream definition XML.
#[derive(Debug, Clone)]
pub struct PrefixUnitCatalog {
    prefixes: PrefixCatalog,
    units: UnitCatalog,
    diagnostics: Vec<BuildDiagnostic>,
}

impl PrefixUnitCatalog {
    /// Builds a combined catalog from already-loaded generic definition documents.
    pub fn from_documents<I>(documents: I) -> Self
    where
        I: IntoIterator<Item = DefinitionDocument>,
    {
        let documents = documents.into_iter().collect::<Vec<_>>();
        let mut diagnostics = Vec::new();
        let mut prefixes = Vec::new();
        let mut units = Vec::new();

        for document in &documents {
            let mut prefix_catalog =
                PrefixCatalog::from_document_with_diagnostics(document, &mut diagnostics);
            prefixes.append(&mut prefix_catalog.prefixes);

            let mut unit_catalog =
                UnitCatalog::from_document_with_diagnostics(document, &mut diagnostics);
            units.append(&mut unit_catalog.units);
        }

        Self {
            prefixes: PrefixCatalog { prefixes },
            units: UnitCatalog { units },
            diagnostics,
        }
    }

    /// Builds a combined catalog from named XML string sources.
    pub fn from_xml_sources<I, S, X>(sources: I) -> Self
    where
        I: IntoIterator<Item = (S, X)>,
        S: Into<crate::definitions::DefinitionSource>,
        X: AsRef<str>,
    {
        let documents = sources
            .into_iter()
            .map(|(source, xml)| load_definition_xml_str(source, xml.as_ref()));
        Self::from_documents(documents)
    }

    /// Returns the loaded prefix catalog.
    pub fn prefixes(&self) -> &PrefixCatalog {
        &self.prefixes
    }

    /// Returns the loaded unit catalog.
    pub fn units(&self) -> &UnitCatalog {
        &self.units
    }

    /// Returns recoverable typed-builder diagnostics.
    pub fn diagnostics(&self) -> &[BuildDiagnostic] {
        &self.diagnostics
    }

    /// Finds a prefix definition by any of its loaded names.
    pub fn prefix_by_name(&self, name: &str) -> Option<&PrefixDefinition> {
        self.prefixes.find_by_name(name)
    }

    /// Finds a unit definition by any of its loaded names.
    pub fn unit_by_name(&self, name: &str) -> Option<&UnitDefinition> {
        self.units.find_by_name(name)
    }

    /// Registers all active loaded prefix and unit names into a parser registry.
    pub fn register_into(&self, registry: &mut StaticRegistry) {
        self.prefixes.register_all(registry);
        self.units.register_all(registry);
    }
}

/// Loads the upstream prefix, currency, and unit XML files from a data directory.
pub fn load_prefix_unit_catalog_from_dir(
    data_dir: impl AsRef<Path>,
) -> Result<PrefixUnitCatalog, DefinitionIoError> {
    let data_dir = data_dir.as_ref();
    let prefixes = load_definition_xml_file(data_dir.join("prefixes.xml.in"))?;
    let currencies = load_definition_xml_file(data_dir.join("currencies.xml.in"))?;
    let units = load_definition_xml_file(data_dir.join("units.xml.in"))?;
    Ok(PrefixUnitCatalog::from_documents([
        prefixes, currencies, units,
    ]))
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

fn parse_i32_attribute(field: &crate::definitions::DefinitionField, name: &str) -> Option<i32> {
    field
        .attributes()
        .get(name)
        .and_then(|value| value.parse().ok())
}

fn parse_comma_separated_metadata(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(String::from)
        .collect()
}
