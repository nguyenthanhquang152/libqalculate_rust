//! Generic XML definition loader core.
//!
//! Upstream oracle:
//! - `../libqalculate/libqalculate/Calculator-definitions.cc`
//! - `../libqalculate/data/prefixes.xml.in`
//! - `../libqalculate/data/units.xml.in`
//! - `../libqalculate/data/functions.xml.in`
//! - `../libqalculate/data/variables.xml.in`
//! - `../libqalculate/data/datasets.xml.in`
//!
//! This module preserves loader structure, provenance, and recoverable
//! diagnostics only. Typed construction of units, prefixes, functions,
//! variables, datasets, currencies, rates, and conversions belongs to the
//! downstream Epic 9 tasks that consume this loader.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::Path;

/// Source identity recorded on every loaded definition node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DefinitionSource {
    name: String,
}

impl DefinitionSource {
    /// Creates a new source identity from a display name or path.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// Returns the source name used in diagnostics and provenance.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl From<&str> for DefinitionSource {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for DefinitionSource {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Approximate XML location and category context for a loaded node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionProvenance {
    source: DefinitionSource,
    line: usize,
    column: usize,
    byte_offset: usize,
    category_path: Vec<String>,
}

impl DefinitionProvenance {
    fn new(
        source: DefinitionSource,
        line: usize,
        column: usize,
        byte_offset: usize,
        category_path: Vec<String>,
    ) -> Self {
        Self {
            source,
            line,
            column,
            byte_offset,
            category_path,
        }
    }

    /// Returns the XML source identity.
    pub fn source(&self) -> &DefinitionSource {
        &self.source
    }

    /// Returns the one-based XML line number when available.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Returns the one-based XML column number when available.
    pub fn column(&self) -> usize {
        self.column
    }

    /// Returns the byte offset in the source XML when available.
    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    /// Returns the category path active at this XML node.
    pub fn category_path(&self) -> &[String] {
        &self.category_path
    }
}

/// Severity level for loader diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DefinitionSeverity {
    /// Informational loader note.
    Information,
    /// Recoverable loader warning.
    Warning,
    /// Loader error that prevented all or part of a document from loading.
    Error,
}

/// Machine-readable diagnostic category emitted by the definition loader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefinitionDiagnosticKind {
    /// XML could not be parsed.
    MalformedXml,
    /// The XML document root is not a `QALCULATE` element.
    MissingRoot,
    /// A tag appeared where this generic loader has no definition-item role for it.
    UnsupportedTag,
    /// Two loaded items expose the same parsed name in one document.
    DuplicateName,
}

/// Structured warning or error emitted while loading definition XML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionDiagnostic {
    severity: DefinitionSeverity,
    kind: DefinitionDiagnosticKind,
    message: String,
    source: DefinitionSource,
    line: usize,
    column: usize,
    byte_offset: usize,
    tag: Option<String>,
    name: Option<String>,
    category_path: Vec<String>,
}

impl DefinitionDiagnostic {
    fn from_provenance(
        severity: DefinitionSeverity,
        kind: DefinitionDiagnosticKind,
        message: impl Into<String>,
        provenance: &DefinitionProvenance,
        tag: Option<String>,
        name: Option<String>,
    ) -> Self {
        Self {
            severity,
            kind,
            message: message.into(),
            source: provenance.source.clone(),
            line: provenance.line,
            column: provenance.column,
            byte_offset: provenance.byte_offset,
            tag,
            name,
            category_path: provenance.category_path.clone(),
        }
    }

    fn parse_error(
        source: DefinitionSource,
        line: usize,
        column: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: DefinitionSeverity::Error,
            kind: DefinitionDiagnosticKind::MalformedXml,
            message: message.into(),
            source,
            line,
            column,
            byte_offset: 0,
            tag: None,
            name: None,
            category_path: Vec::new(),
        }
    }

    /// Returns the diagnostic severity.
    pub fn severity(&self) -> DefinitionSeverity {
        self.severity
    }

    /// Returns the machine-readable diagnostic kind.
    pub fn kind(&self) -> DefinitionDiagnosticKind {
        self.kind
    }

    /// Returns the human-readable diagnostic message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the XML source identity.
    pub fn source(&self) -> &DefinitionSource {
        &self.source
    }

    /// Returns the one-based XML line number when available.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Returns the one-based XML column number when available.
    pub fn column(&self) -> usize {
        self.column
    }

    /// Returns the byte offset in the source XML when available.
    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    /// Returns the XML tag tied to this diagnostic, if one exists.
    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    /// Returns the parsed definition name tied to this diagnostic, if one exists.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the category path active at the diagnostic location.
    pub fn category_path(&self) -> &[String] {
        &self.category_path
    }
}

/// Loaded definition document plus recoverable diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionDocument {
    source: DefinitionSource,
    version: Option<String>,
    categories: Vec<DefinitionCategory>,
    items: Vec<DefinitionItem>,
    actions: Vec<DefinitionAction>,
    diagnostics: Vec<DefinitionDiagnostic>,
}

impl DefinitionDocument {
    fn empty(source: DefinitionSource) -> Self {
        Self {
            source,
            version: None,
            categories: Vec::new(),
            items: Vec::new(),
            actions: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Returns the XML source identity.
    pub fn source(&self) -> &DefinitionSource {
        &self.source
    }

    /// Returns the `QALCULATE` version attribute, if present.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    /// Returns the categories found while traversing the document.
    pub fn categories(&self) -> &[DefinitionCategory] {
        &self.categories
    }

    /// Returns the flattened definition items found in document order.
    pub fn items(&self) -> &[DefinitionItem] {
        &self.items
    }

    /// Returns activation/deactivation actions found in document order.
    pub fn actions(&self) -> &[DefinitionAction] {
        &self.actions
    }

    /// Returns loader diagnostics found while parsing and traversing XML.
    pub fn diagnostics(&self) -> &[DefinitionDiagnostic] {
        &self.diagnostics
    }
}

/// A category node and the path implied by nested upstream `<category>` elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionCategory {
    title: Option<String>,
    path: Vec<String>,
    provenance: DefinitionProvenance,
}

impl DefinitionCategory {
    /// Returns the category title after removing upstream translation markers.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns the full nested category path.
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// Returns source provenance for this category.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }
}

/// Supported top-level definition item families in upstream XML.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DefinitionItemKind {
    /// User-defined function definition.
    Function,
    /// Built-in function metadata definition.
    BuiltinFunction,
    /// Data set definition.
    Dataset,
    /// Built-in data set metadata definition.
    BuiltinDataset,
    /// Known variable definition.
    Variable,
    /// Built-in variable metadata definition.
    BuiltinVariable,
    /// Unknown variable definition.
    UnknownVariable,
    /// Unit definition.
    Unit,
    /// Built-in unit metadata definition.
    BuiltinUnit,
    /// Prefix definition.
    Prefix,
    /// Dataset object row from files such as `elements.xml.in` and `planets.xml.in`.
    DataObject,
}

impl DefinitionItemKind {
    fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "function" => Some(Self::Function),
            "builtin_function" => Some(Self::BuiltinFunction),
            "dataset" => Some(Self::Dataset),
            "builtin_dataset" => Some(Self::BuiltinDataset),
            "variable" => Some(Self::Variable),
            "builtin_variable" => Some(Self::BuiltinVariable),
            "unknown" => Some(Self::UnknownVariable),
            "unit" => Some(Self::Unit),
            "builtin_unit" => Some(Self::BuiltinUnit),
            "prefix" => Some(Self::Prefix),
            "object" => Some(Self::DataObject),
            _ => None,
        }
    }
}

/// Generic loaded definition item with raw XML fields preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionItem {
    kind: DefinitionItemKind,
    tag: String,
    attributes: BTreeMap<String, String>,
    names: Vec<String>,
    active: bool,
    active_specified: bool,
    category_path: Vec<String>,
    fields: Vec<DefinitionField>,
    provenance: DefinitionProvenance,
}

impl DefinitionItem {
    /// Returns the normalized item family.
    pub fn kind(&self) -> DefinitionItemKind {
        self.kind.clone()
    }

    /// Returns the source XML tag name.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Returns source XML attributes on the item node.
    pub fn attributes(&self) -> &BTreeMap<String, String> {
        &self.attributes
    }

    /// Returns parsed item names in document order.
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// Returns the effective active flag; missing upstream `active` means active.
    pub fn active(&self) -> bool {
        self.active
    }

    /// Returns true when the XML item explicitly had an `active` attribute.
    pub fn active_specified(&self) -> bool {
        self.active_specified
    }

    /// Returns the category path active when this item was loaded.
    pub fn category_path(&self) -> &[String] {
        &self.category_path
    }

    /// Returns raw XML child fields on the item node.
    pub fn fields(&self) -> &[DefinitionField] {
        &self.fields
    }

    /// Returns the first raw XML child field with the requested tag.
    pub fn field(&self, tag: &str) -> Option<&DefinitionField> {
        self.fields.iter().find(|field| field.tag == tag)
    }

    /// Returns source provenance for this item.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }
}

/// Raw XML child field preserved for a loaded definition item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionField {
    tag: String,
    attributes: BTreeMap<String, String>,
    text: Option<String>,
    fields: Vec<DefinitionField>,
    provenance: DefinitionProvenance,
}

impl DefinitionField {
    /// Returns the source XML tag name.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Returns source XML attributes on the field node.
    pub fn attributes(&self) -> &BTreeMap<String, String> {
        &self.attributes
    }

    /// Returns direct text content after trimming whitespace.
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Returns nested raw XML child fields.
    pub fn fields(&self) -> &[DefinitionField] {
        &self.fields
    }

    /// Returns the first nested raw XML child field with the requested tag.
    pub fn field(&self, tag: &str) -> Option<&DefinitionField> {
        self.fields.iter().find(|field| field.tag == tag)
    }

    /// Returns source provenance for this field.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }
}

/// Activation action kind from upstream `<activate>` and `<deactivate>` nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefinitionActionKind {
    /// Activates a previously inactive non-local item.
    Activate,
    /// Deactivates an active non-local item.
    Deactivate,
}

/// A preserved activation/deactivation instruction from definition XML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionAction {
    kind: DefinitionActionKind,
    name: String,
    category_path: Vec<String>,
    provenance: DefinitionProvenance,
}

impl DefinitionAction {
    /// Returns the activation action kind.
    pub fn kind(&self) -> DefinitionActionKind {
        self.kind
    }

    /// Returns the referenced item name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the category path active when this action was loaded.
    pub fn category_path(&self) -> &[String] {
        &self.category_path
    }

    /// Returns source provenance for this action.
    pub fn provenance(&self) -> &DefinitionProvenance {
        &self.provenance
    }
}

/// Error returned when a definition XML file cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionIoError {
    message: String,
}

impl DefinitionIoError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DefinitionIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DefinitionIoError {}

/// Loads definition XML from a file path.
///
/// File I/O failures are returned as `Err`. XML parse/schema problems are
/// represented as structured diagnostics in the returned document.
pub fn load_definition_xml_file(
    path: impl AsRef<Path>,
) -> Result<DefinitionDocument, DefinitionIoError> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path).map_err(|error| {
        DefinitionIoError::new(format!(
            "failed to read definition XML from {}: {error}",
            path.display()
        ))
    })?;
    Ok(load_definition_xml_str(
        path.display().to_string(),
        &contents,
    ))
}

/// Loads definition XML from a string.
///
/// Malformed XML and unsupported document structure are captured as
/// diagnostics in the returned document instead of panicking.
pub fn load_definition_xml_str(
    source: impl Into<DefinitionSource>,
    xml: &str,
) -> DefinitionDocument {
    let source = source.into();
    let mut loader = DefinitionLoader::new(source, xml);
    loader.load()
}

struct DefinitionLoader<'xml> {
    source: DefinitionSource,
    xml: &'xml str,
    categories: Vec<DefinitionCategory>,
    items: Vec<DefinitionItem>,
    actions: Vec<DefinitionAction>,
    diagnostics: Vec<DefinitionDiagnostic>,
    seen_names: HashMap<String, DefinitionProvenance>,
}

impl<'xml> DefinitionLoader<'xml> {
    fn new(source: DefinitionSource, xml: &'xml str) -> Self {
        Self {
            source,
            xml,
            categories: Vec::new(),
            items: Vec::new(),
            actions: Vec::new(),
            diagnostics: Vec::new(),
            seen_names: HashMap::new(),
        }
    }

    fn load(&mut self) -> DefinitionDocument {
        let parsed = match roxmltree::Document::parse(self.xml) {
            Ok(parsed) => parsed,
            Err(error) => {
                let pos = error.pos();
                let mut document = DefinitionDocument::empty(self.source.clone());
                document.diagnostics.push(DefinitionDiagnostic::parse_error(
                    self.source.clone(),
                    pos.row as usize,
                    pos.col as usize,
                    format!("Malformed XML: {error}"),
                ));
                return document;
            }
        };

        let root = parsed.root_element();
        if root.tag_name().name() != "QALCULATE" {
            let provenance = provenance_for(&parsed, root, &self.source, &[]);
            let mut document = DefinitionDocument::empty(self.source.clone());
            document
                .diagnostics
                .push(DefinitionDiagnostic::from_provenance(
                    DefinitionSeverity::Error,
                    DefinitionDiagnosticKind::MissingRoot,
                    "File not identified as Qalculate definitions XML: missing QALCULATE root",
                    &provenance,
                    Some(root.tag_name().name().to_string()),
                    None,
                ));
            return document;
        }

        let version = root.attribute("version").map(ToOwned::to_owned);
        self.parse_container(&parsed, root, &[]);

        DefinitionDocument {
            source: self.source.clone(),
            version,
            categories: std::mem::take(&mut self.categories),
            items: std::mem::take(&mut self.items),
            actions: std::mem::take(&mut self.actions),
            diagnostics: std::mem::take(&mut self.diagnostics),
        }
    }

    fn parse_container(
        &mut self,
        document: &roxmltree::Document<'_>,
        node: roxmltree::Node<'_, '_>,
        parent_path: &[String],
    ) {
        let mut category_path = parent_path.to_vec();
        if node.tag_name().name() == "category" {
            let title = category_title(node).map(normalize_translated_label);
            if let Some(title) = title.as_ref().filter(|title| !title.is_empty()) {
                category_path.push(title.clone());
            }
            let provenance = provenance_for(document, node, &self.source, &category_path);
            self.categories.push(DefinitionCategory {
                title,
                path: category_path.clone(),
                provenance,
            });
        }

        for child in element_children(node) {
            let tag = child.tag_name().name();
            if tag == "title" && node.tag_name().name() == "category" {
                continue;
            }
            if tag == "category" {
                self.parse_container(document, child, &category_path);
            } else if let Some(kind) = DefinitionItemKind::from_tag(tag) {
                self.parse_item(document, child, kind, &category_path);
            } else if let Some(kind) = action_kind(tag) {
                self.parse_action(document, child, kind, &category_path);
            } else {
                let provenance = provenance_for(document, child, &self.source, &category_path);
                self.diagnostics.push(DefinitionDiagnostic::from_provenance(
                    DefinitionSeverity::Warning,
                    DefinitionDiagnosticKind::UnsupportedTag,
                    format!("Unsupported definition XML tag <{tag}>"),
                    &provenance,
                    Some(tag.to_string()),
                    None,
                ));
            }
        }
    }

    fn parse_item(
        &mut self,
        document: &roxmltree::Document<'_>,
        node: roxmltree::Node<'_, '_>,
        kind: DefinitionItemKind,
        category_path: &[String],
    ) {
        let attributes = attributes_for(node);
        let active_specified = attributes.contains_key("active");
        let active = attributes
            .get("active")
            .is_none_or(|value| value != "false");
        let fields = element_children(node)
            .map(|child| parse_field(document, child, &self.source, category_path))
            .collect::<Vec<_>>();
        let mut names = Vec::new();
        if let Some(name) = attributes.get("name").and_then(|name| normalize_name(name)) {
            names.push(name);
        }
        collect_names_from_fields(&fields, &mut names);
        dedup_preserving_order(&mut names);

        let provenance = provenance_for(document, node, &self.source, category_path);
        for name in &names {
            if self.seen_names.contains_key(name) {
                self.diagnostics.push(DefinitionDiagnostic::from_provenance(
                    DefinitionSeverity::Warning,
                    DefinitionDiagnosticKind::DuplicateName,
                    format!("Duplicate definition name `{name}`"),
                    &provenance,
                    Some(node.tag_name().name().to_string()),
                    Some(name.clone()),
                ));
            } else {
                self.seen_names.insert(name.clone(), provenance.clone());
            }
        }

        self.items.push(DefinitionItem {
            kind,
            tag: node.tag_name().name().to_string(),
            attributes,
            names,
            active,
            active_specified,
            category_path: category_path.to_vec(),
            fields,
            provenance,
        });
    }

    fn parse_action(
        &mut self,
        document: &roxmltree::Document<'_>,
        node: roxmltree::Node<'_, '_>,
        kind: DefinitionActionKind,
        category_path: &[String],
    ) {
        let provenance = provenance_for(document, node, &self.source, category_path);
        self.actions.push(DefinitionAction {
            kind,
            name: direct_text(node).unwrap_or_default(),
            category_path: category_path.to_vec(),
            provenance,
        });
    }
}

fn action_kind(tag: &str) -> Option<DefinitionActionKind> {
    match tag {
        "activate" => Some(DefinitionActionKind::Activate),
        "deactivate" => Some(DefinitionActionKind::Deactivate),
        _ => None,
    }
}

fn element_children<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
) -> impl Iterator<Item = roxmltree::Node<'a, 'input>> {
    node.children().filter(roxmltree::Node::is_element)
}

fn attributes_for(node: roxmltree::Node<'_, '_>) -> BTreeMap<String, String> {
    node.attributes()
        .map(|attribute| {
            (
                attribute.name().to_string(),
                attribute.value().trim().to_string(),
            )
        })
        .collect()
}

fn category_title(node: roxmltree::Node<'_, '_>) -> Option<String> {
    element_children(node)
        .find(|child| child.tag_name().name() == "title")
        .and_then(direct_text)
}

fn normalize_translated_label(raw: String) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with('!') {
        return trimmed.to_string();
    }
    match trimmed[1..].find('!') {
        Some(index) => trimmed[index + 2..].to_string(),
        None => trimmed.to_string(),
    }
}

fn parse_field(
    document: &roxmltree::Document<'_>,
    node: roxmltree::Node<'_, '_>,
    source: &DefinitionSource,
    category_path: &[String],
) -> DefinitionField {
    DefinitionField {
        tag: node.tag_name().name().to_string(),
        attributes: attributes_for(node),
        text: direct_text(node),
        fields: element_children(node)
            .map(|child| parse_field(document, child, source, category_path))
            .collect(),
        provenance: provenance_for(document, node, source, category_path),
    }
}

fn direct_text(node: roxmltree::Node<'_, '_>) -> Option<String> {
    let mut text = String::new();
    for child in node.children().filter(roxmltree::Node::is_text) {
        if let Some(child_text) = child.text() {
            text.push_str(child_text);
        }
    }
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn collect_names_from_fields(fields: &[DefinitionField], names: &mut Vec<String>) {
    for field in fields {
        match field.tag.as_str() {
            "names" => {
                if let Some(text) = field.text() {
                    names.extend(text.split(',').filter_map(normalize_name));
                }
            }
            "name" | "abbreviation" | "plural" => {
                if let Some(text) = field.text().and_then(normalize_name) {
                    names.push(text);
                } else if let Some(text) = field
                    .field("name")
                    .and_then(DefinitionField::text)
                    .and_then(normalize_name)
                {
                    names.push(text);
                }
            }
            _ => {}
        }
    }
}

fn normalize_name(raw: &str) -> Option<String> {
    let normalized = normalize_translated_label(raw.trim().to_string());
    let mut name = normalized.trim();
    if name.is_empty() {
        return None;
    }
    if let Some((_, suffix)) = name.rsplit_once(':') {
        name = suffix.trim();
    }
    name = name.trim_start_matches('-').trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn dedup_preserving_order(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn provenance_for(
    document: &roxmltree::Document<'_>,
    node: roxmltree::Node<'_, '_>,
    source: &DefinitionSource,
    category_path: &[String],
) -> DefinitionProvenance {
    let offset = node.range().start;
    let position = document.text_pos_at(offset);
    DefinitionProvenance::new(
        source.clone(),
        position.row as usize,
        position.col as usize,
        offset,
        category_path.to_vec(),
    )
}
