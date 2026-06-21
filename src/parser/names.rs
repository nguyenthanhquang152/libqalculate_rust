//! Name resolution for functions, units, variables, and prefixes.
//!
//! This module provides the [`NameRegistry`] trait and supporting types
//! for parser name resolution.  The registry interface is intentionally
//! thin — it supports lookup by name and returns classification metadata,
//! but does **not** own full definition semantics (evaluation, conversion,
//! etc.), which belong to later porting tasks.
//!
//! # Upstream reference
//!
//! Upstream uses pre-sorted `ufv[type][length]` lookup tables in
//! `Calculator-parse.cc` (lines ~2806-3880) with priority:
//!
//! 1. User-defined local names (highest)
//! 2. Prefixes (`ufv[0]`)
//! 3. Functions (`ufv[1]`)
//! 4. Units (`ufv[2]`)
//! 5. Variables (`ufv[3]`)
//!
//! Within the same length, longer matches win.  For equal-length
//! matches, the type priority above applies.  Functions matched
//! without a following `(` get lower priority than variables/units
//! of equal or greater length.

use crate::ast::{DefinitionKind, DefinitionRef};

/// Classification of a matched name from the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameMatch {
    /// Matched a function name.
    Function {
        /// Stable reference for this function definition.
        definition: DefinitionRef,
        /// Minimum number of arguments required (0 if unknown).
        min_args: usize,
        /// Maximum number of arguments accepted (`None` = variadic).
        max_args: Option<usize>,
    },
    /// Matched a unit name (possibly with a prefix).
    Unit {
        /// Stable reference for this unit definition.
        definition: DefinitionRef,
        /// If a prefix was consumed, its stable reference.
        prefix: Option<DefinitionRef>,
    },
    /// Matched a variable name.
    Variable {
        /// Stable reference for this variable definition.
        definition: DefinitionRef,
    },
    /// Matched a standalone prefix (not followed by a unit).
    Prefix {
        /// Stable reference for this prefix definition.
        definition: DefinitionRef,
    },
}

/// Trait for name resolution during parsing.
///
/// Implementations provide lookup of names against function, unit,
/// variable, and prefix registries.  The parser calls [`lookup`]
/// at each identifier token and uses the result to decide the AST
/// node type.
///
/// The default implementation (no registry) returns `None` for all
/// names, causing identifiers to fall through to `Symbolic` nodes.
///
/// [`lookup`]: NameRegistry::lookup
pub trait NameRegistry {
    /// Look up a name and return its classification, or `None` if
    /// the name is not found in any registry.
    ///
    /// When `followed_by_paren` is `true`, the parser has already
    /// seen `name(` — this gives function matches higher priority
    /// per upstream rules.
    fn lookup(&self, name: &str, followed_by_paren: bool) -> Option<NameMatch>;
}

/// A no-op registry that resolves no names.
///
/// This is the default when no definition data is loaded.  All
/// identifiers become `Symbolic` AST nodes and function calls
/// use unresolved `FunctionRef` handles.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmptyRegistry;

impl NameRegistry for EmptyRegistry {
    fn lookup(&self, _name: &str, _followed_by_paren: bool) -> Option<NameMatch> {
        None
    }
}

/// A static registry built from explicit name lists.
///
/// Useful for test fixtures and minimal parser configurations
/// without loading full upstream XML data.
#[derive(Debug, Clone, Default)]
pub struct StaticRegistry {
    entries: Vec<StaticEntry>,
}

/// A single entry in a [`StaticRegistry`].
#[derive(Debug, Clone)]
struct StaticEntry {
    /// The name to match (case-sensitive).
    name: String,
    /// The kind of match this entry produces.
    kind: StaticEntryKind,
}

/// Classification for a static registry entry.
#[derive(Debug, Clone)]
enum StaticEntryKind {
    Function {
        min_args: usize,
        max_args: Option<usize>,
    },
    Unit,
    Variable,
    Prefix,
}

impl StaticEntryKind {
    /// Priority for disambiguation when multiple entries match.
    ///
    /// Higher values win.  Order matches upstream:
    /// Function (3) > Unit (2) > Variable (1) > Prefix (0).
    fn priority(&self) -> u8 {
        match self {
            Self::Function { .. } => 3,
            Self::Unit => 2,
            Self::Variable => 1,
            Self::Prefix => 0,
        }
    }
}

impl StaticRegistry {
    /// Creates an empty static registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a function name with argument bounds.
    pub fn add_function(
        &mut self,
        name: impl Into<String>,
        min_args: usize,
        max_args: Option<usize>,
    ) {
        self.entries.push(StaticEntry {
            name: name.into(),
            kind: StaticEntryKind::Function { min_args, max_args },
        });
    }

    /// Registers a unit name.
    pub fn add_unit(&mut self, name: impl Into<String>) {
        self.entries.push(StaticEntry {
            name: name.into(),
            kind: StaticEntryKind::Unit,
        });
    }

    /// Registers a variable name.
    pub fn add_variable(&mut self, name: impl Into<String>) {
        self.entries.push(StaticEntry {
            name: name.into(),
            kind: StaticEntryKind::Variable,
        });
    }

    /// Registers a prefix name.
    pub fn add_prefix(&mut self, name: impl Into<String>) {
        self.entries.push(StaticEntry {
            name: name.into(),
            kind: StaticEntryKind::Prefix,
        });
    }
}

impl NameRegistry for StaticRegistry {
    fn lookup(&self, name: &str, _followed_by_paren: bool) -> Option<NameMatch> {
        // Priority order per upstream: function > unit > variable > prefix.
        // Longest match wins, but since StaticRegistry entries are exact-match,
        // we just search in priority order.
        let mut best: Option<(&StaticEntry, u8)> = None;

        for entry in &self.entries {
            if name == entry.name {
                let priority = entry.kind.priority();
                if best.as_ref().is_none_or(|(_, p)| priority > *p) {
                    best = Some((entry, priority));
                }
            }
        }

        // Also check if name starts with a known prefix followed by a known unit.
        if best.is_none() {
            for prefix_entry in self
                .entries
                .iter()
                .filter(|e| matches!(e.kind, StaticEntryKind::Prefix))
            {
                if let Some(rest) = name.strip_prefix(&prefix_entry.name) {
                    if !rest.is_empty() {
                        for unit_entry in self
                            .entries
                            .iter()
                            .filter(|e| matches!(e.kind, StaticEntryKind::Unit))
                        {
                            if rest == unit_entry.name {
                                return Some(NameMatch::Unit {
                                    definition: DefinitionRef::new(
                                        DefinitionKind::Unit,
                                        unit_entry.name.clone(),
                                    ),
                                    prefix: Some(DefinitionRef::new(
                                        DefinitionKind::Prefix,
                                        prefix_entry.name.clone(),
                                    )),
                                });
                            }
                        }
                    }
                }
            }
        }

        best.map(|(entry, _)| match &entry.kind {
            StaticEntryKind::Function { min_args, max_args } => NameMatch::Function {
                definition: DefinitionRef::new(DefinitionKind::Function, entry.name.clone()),
                min_args: *min_args,
                max_args: *max_args,
            },
            StaticEntryKind::Unit => NameMatch::Unit {
                definition: DefinitionRef::new(DefinitionKind::Unit, entry.name.clone()),
                prefix: None,
            },
            StaticEntryKind::Variable => NameMatch::Variable {
                definition: DefinitionRef::new(DefinitionKind::Variable, entry.name.clone()),
            },
            StaticEntryKind::Prefix => NameMatch::Prefix {
                definition: DefinitionRef::new(DefinitionKind::Prefix, entry.name.clone()),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry_returns_none() {
        let reg = EmptyRegistry;
        assert_eq!(reg.lookup("sin", true), None);
        assert_eq!(reg.lookup("m", false), None);
    }

    #[test]
    fn static_registry_matches_function() {
        let mut reg = StaticRegistry::new();
        reg.add_function("sin", 1, Some(1));
        let result = reg.lookup("sin", true);
        assert!(matches!(result, Some(NameMatch::Function { .. })));
    }

    #[test]
    fn static_registry_matches_unit() {
        let mut reg = StaticRegistry::new();
        reg.add_unit("m");
        let result = reg.lookup("m", false);
        assert!(matches!(result, Some(NameMatch::Unit { prefix: None, .. })));
    }

    #[test]
    fn static_registry_matches_variable() {
        let mut reg = StaticRegistry::new();
        reg.add_variable("alpha");
        let result = reg.lookup("alpha", false);
        assert!(matches!(result, Some(NameMatch::Variable { .. })));
    }

    #[test]
    fn static_registry_resolves_prefix_plus_unit() {
        let mut reg = StaticRegistry::new();
        reg.add_prefix("k");
        reg.add_unit("m");
        let result = reg.lookup("km", false);
        assert!(
            matches!(
                result,
                Some(NameMatch::Unit {
                    prefix: Some(_),
                    ..
                })
            ),
            "expected prefixed unit, got {result:?}"
        );
    }

    #[test]
    fn static_registry_function_wins_over_variable() {
        let mut reg = StaticRegistry::new();
        reg.add_variable("sin");
        reg.add_function("sin", 1, Some(1));
        let result = reg.lookup("sin", true);
        assert!(matches!(result, Some(NameMatch::Function { .. })));
    }

    #[test]
    fn static_registry_unknown_returns_none() {
        let reg = StaticRegistry::new();
        assert_eq!(reg.lookup("xyz", false), None);
    }
}
