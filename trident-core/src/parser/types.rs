use serde::Serialize;

/// Source location span for code generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Span {
    pub start_line: usize,
    pub end_line: usize,
}

/// A comment line, preserving exact whitespace
#[derive(Debug, Clone, Serialize)]
pub struct CommentAst {
    /// The prefix before %% (whitespace/newlines preserved)
    pub prefix: String,
    /// The text after %% (including any leading space)
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileAst {
    /// Layout algorithm to use: "hierarchical" (default) or "grid"
    pub layout: Option<String>,
    pub items: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Stmt {
    Group(GroupAst),
    Node(NodeAst),
    Relation(RelationAst),
    Comment(CommentAst),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Ident(pub String);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub struct PointI {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupAst {
    /// None => anonymous `group { ... }`
    pub id: Option<Ident>,
    /// local position relative to closest parent group (or root)
    pub pos: Option<PointI>,
    pub items: Vec<Stmt>,
    /// Source span for round-tripping
    pub span: Option<Span>,
}

/// A node declaration (class, interface, enum, etc.)
#[derive(Debug, Clone, Serialize)]
pub struct NodeAst {
    /// Node kind: "class", "node"
    pub kind: String,
    /// Original kind keyword as written by user (e.g., "enum", "diamond")
    /// Used for code regeneration to preserve user's syntax
    pub original_kind: String,
    /// Modifiers: "abstract", "interface", "enum", "rectangle", "circle", "diamond", etc.
    pub modifiers: Vec<String>,
    /// Unique identifier
    pub id: Ident,
    /// Display label (optional)
    pub label: Option<String>,
    /// local position relative to closest parent group (or root)
    pub pos: Option<PointI>,
    /// Custom width (from @width directive)
    pub width: Option<i32>,
    /// Custom height (from @height directive)
    pub height: Option<i32>,
    /// opaque lines inside node block (renderer decides)
    pub body_lines: Vec<String>,
    /// Source span for round-tripping
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelationAst {
    pub from: Ident,
    /// Arrow as canonical string name (e.g., "extends_left", "assoc_right")
    pub arrow: String,
    pub to: Ident,
    pub label: Option<String>,
    /// Source span for round-tripping
    pub span: Option<Span>,
}

// ============================================================================
// Arrow Registry - Single source of truth for arrow definitions
// ============================================================================

/// Line style for arrow rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LineStyle {
    Solid,
    Dashed,
}

/// Head/marker style for arrow endpoints
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadStyle {
    /// No marker (plain line end)
    None,
    /// Simple arrowhead (>)
    Arrow,
    /// Hollow triangle (inheritance/extends)
    Triangle,
    /// Filled diamond (composition)
    DiamondFilled,
    /// Hollow diamond (aggregation)
    DiamondEmpty,
}

/// Direction of an arrow (affects layout hierarchy)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArrowDirection {
    /// Arrow points from left to right (A --> B means A points to B)
    Right,
    /// Arrow points from right to left (A <-- B means B points to A)  
    Left,
    /// Non-directional (line between A and B)
    None,
}

/// Complete definition of an arrow type
#[derive(Debug, Clone, Serialize)]
pub struct ArrowDefinition {
    /// The token as written in source (e.g., "-->", "<|--")
    pub token: &'static str,
    /// User-facing short name for the arrow (e.g., "assoc", "extends")
    /// Direction suffix (_left/_right) is added automatically for directional arrows
    pub name: &'static str,
    /// Detailed description for autocomplete
    pub detail: &'static str,
    /// Line style (solid or dashed)
    pub line_style: LineStyle,
    /// Head style at the "to" end (or source for left arrows)
    /// For diamonds at source (composition/aggregation), this is the marker at the "from" node
    pub head_style: HeadStyle,
    /// Direction of the arrow
    pub direction: ArrowDirection,
    /// For hierarchical layout: does this arrow indicate parent->child relationship?
    /// If true, the "to" node is considered a child of "from" in layout
    pub is_hierarchy_edge: bool,
    /// For hierarchical layout: is the direction reversed?
    /// If true, "from" is the child and "to" is the parent (like extends)
    pub hierarchy_reversed: bool,
}

/// Arrow definitions - single source of truth
/// Arrows are defined in their canonical "right" form (A --> B)
/// Left variants are generated automatically by the system
pub const ARROW_DEFINITIONS: &[ArrowDefinition] = &[
    // Association: simple arrow
    ArrowDefinition {
        token: "-->",
        name: "assoc",
        detail: "Association arrow",
        line_style: LineStyle::Solid,
        head_style: HeadStyle::Arrow,
        direction: ArrowDirection::Right,
        is_hierarchy_edge: true,
        hierarchy_reversed: false,
    },
    // Extends/Inheritance: hollow triangle
    ArrowDefinition {
        token: "--|>",
        name: "extends",
        detail: "Inheritance/extends arrow",
        line_style: LineStyle::Solid,
        head_style: HeadStyle::Triangle,
        direction: ArrowDirection::Right,
        is_hierarchy_edge: true,
        hierarchy_reversed: true, // Child extends Parent, so "to" is parent
    },
    // Implements/Realizes: dashed with hollow triangle
    ArrowDefinition {
        token: "..|>",
        name: "implements",
        detail: "Implements/realizes arrow",
        line_style: LineStyle::Dashed,
        head_style: HeadStyle::Triangle,
        direction: ArrowDirection::Right,
        is_hierarchy_edge: true,
        hierarchy_reversed: true, // Implementor implements Interface, so "to" is parent
    },
    // Dependency: dashed arrow
    ArrowDefinition {
        token: "..>",
        name: "dep",
        detail: "Dependency arrow",
        line_style: LineStyle::Dashed,
        head_style: HeadStyle::Arrow,
        direction: ArrowDirection::Right,
        is_hierarchy_edge: true,
        hierarchy_reversed: false,
    },
    // Composition: filled diamond at source
    ArrowDefinition {
        token: "*--",
        name: "compose",
        detail: "Composition (strong ownership)",
        line_style: LineStyle::Solid,
        head_style: HeadStyle::DiamondFilled, // Diamond at source (from node)
        direction: ArrowDirection::Right,
        is_hierarchy_edge: true,
        hierarchy_reversed: false,
    },
    // Aggregation: hollow diamond at source
    ArrowDefinition {
        token: "o--",
        name: "aggregate",
        detail: "Aggregation (weak ownership)",
        line_style: LineStyle::Solid,
        head_style: HeadStyle::DiamondEmpty, // Diamond at source (from node)
        direction: ArrowDirection::Right,
        is_hierarchy_edge: true,
        hierarchy_reversed: false,
    },
    // Simple line (non-directional)
    ArrowDefinition {
        token: "---",
        name: "line",
        detail: "Simple line (no direction)",
        line_style: LineStyle::Solid,
        head_style: HeadStyle::None,
        direction: ArrowDirection::None,
        is_hierarchy_edge: false,
        hierarchy_reversed: false,
    },
    // Dotted line (non-directional)
    ArrowDefinition {
        token: "..",
        name: "dotted",
        detail: "Dotted line (no direction)",
        line_style: LineStyle::Dashed,
        head_style: HeadStyle::None,
        direction: ArrowDirection::None,
        is_hierarchy_edge: false,
        hierarchy_reversed: false,
    },
];

/// Runtime arrow entry with resolved canonical name (including direction suffix)
#[derive(Debug, Clone, Serialize)]
pub struct ArrowEntry {
    /// Token string for parsing (e.g., "-->", "<--")
    pub token: &'static str,
    /// Canonical name with direction suffix (e.g., "assoc_right", "assoc_left")
    pub canonical_name: String,
    /// Reference to the base definition
    #[serde(flatten)]
    pub definition: ArrowDefinitionJson,
}

/// JSON-serializable version of ArrowDefinition (for WASM export)
#[derive(Debug, Clone, Serialize)]
pub struct ArrowDefinitionJson {
    pub name: &'static str,
    pub detail: &'static str,
    pub line_style: LineStyle,
    pub head_style: HeadStyle,
    pub direction: ArrowDirection,
    pub is_left: bool,
}

/// Generate the reverse token for a directional arrow
fn reverse_token(token: &str) -> Option<String> {
    // Map of character pairs that reverse
    let reversed: String = token.chars().rev().map(|c| match c {
        '>' => '<',
        '<' => '>',
        _ => c,
    }).collect();
    
    if reversed != token {
        Some(reversed)
    } else {
        None
    }
}

/// Build the complete arrow registry including left variants
/// Returns entries sorted by token length (longest first) for proper parsing
pub fn build_arrow_registry() -> Vec<ArrowEntry> {
    let mut entries = Vec::new();
    
    for def in ARROW_DEFINITIONS {
        match def.direction {
            ArrowDirection::Right => {
                // Add right variant
                entries.push(ArrowEntry {
                    token: def.token,
                    canonical_name: format!("{}_right", def.name),
                    definition: ArrowDefinitionJson {
                        name: def.name,
                        detail: def.detail,
                        line_style: def.line_style,
                        head_style: def.head_style,
                        direction: def.direction,
                        is_left: false,
                    },
                });
                
                // Generate and add left variant
                if let Some(left_token) = reverse_token(def.token) {
                    // We need to leak the string to get a &'static str
                    // This is fine since we only do it once at startup
                    let left_token: &'static str = Box::leak(left_token.into_boxed_str());
                    
                    entries.push(ArrowEntry {
                        token: left_token,
                        canonical_name: format!("{}_left", def.name),
                        definition: ArrowDefinitionJson {
                            name: def.name,
                            detail: def.detail,
                            line_style: def.line_style,
                            head_style: def.head_style,
                            direction: ArrowDirection::Left,
                            is_left: true,
                        },
                    });
                }
            }
            ArrowDirection::Left => {
                // Arrows defined as left-only (rare, but supported)
                entries.push(ArrowEntry {
                    token: def.token,
                    canonical_name: format!("{}_left", def.name),
                    definition: ArrowDefinitionJson {
                        name: def.name,
                        detail: def.detail,
                        line_style: def.line_style,
                        head_style: def.head_style,
                        direction: def.direction,
                        is_left: true,
                    },
                });
            }
            ArrowDirection::None => {
                // Non-directional arrows don't get a suffix
                entries.push(ArrowEntry {
                    token: def.token,
                    canonical_name: def.name.to_string(),
                    definition: ArrowDefinitionJson {
                        name: def.name,
                        detail: def.detail,
                        line_style: def.line_style,
                        head_style: def.head_style,
                        direction: def.direction,
                        is_left: false,
                    },
                });
            }
        }
    }
    
    // Sort by token length (longest first) for proper parsing
    entries.sort_by(|a, b| b.token.len().cmp(&a.token.len()));
    
    entries
}

// Lazy static registry for efficient lookups
use std::sync::LazyLock;

static ARROW_REGISTRY: LazyLock<Vec<ArrowEntry>> = LazyLock::new(build_arrow_registry);

/// Get the complete arrow registry
pub fn get_arrow_registry() -> &'static Vec<ArrowEntry> {
    &ARROW_REGISTRY
}

/// Look up canonical name from token string
pub fn arrow_from_token(token: &str) -> Option<&'static str> {
    ARROW_REGISTRY
        .iter()
        .find(|e| e.token == token)
        .map(|e| {
            // Return a reference to the canonical_name
            // We need to leak this since canonical_name is a String, not &'static str
            // Actually, let's just return the token lookup result differently
            e.canonical_name.as_str()
        })
}

/// Look up token string from canonical name
pub fn token_from_arrow(arrow: &str) -> Option<&'static str> {
    ARROW_REGISTRY
        .iter()
        .find(|e| e.canonical_name == arrow)
        .map(|e| e.token)
}

/// Get arrow definition by canonical name
pub fn get_arrow_definition(canonical_name: &str) -> Option<&'static ArrowEntry> {
    ARROW_REGISTRY
        .iter()
        .find(|e| e.canonical_name == canonical_name)
}

/// Get the base arrow name (without _left/_right suffix)
pub fn get_base_arrow_name(canonical_name: &str) -> &str {
    canonical_name
        .strip_suffix("_left")
        .or_else(|| canonical_name.strip_suffix("_right"))
        .unwrap_or(canonical_name)
}

/// Check if an arrow is a "left" arrow
pub fn is_left_arrow(canonical_name: &str) -> bool {
    canonical_name.ends_with("_left")
}

/// Get all arrow tokens (for parser/tokenizer)
pub fn get_arrow_tokens() -> Vec<&'static str> {
    ARROW_REGISTRY.iter().map(|e| e.token).collect()
}

// ============================================================================
// Known node kinds - for parsing
// ============================================================================

/// The two primary node kinds
pub const KNOWN_NODE_KINDS: &[&str] = &["class", "node"];

/// Keywords that create class kind + add themselves as modifier
pub const CLASS_KEYWORDS: &[&str] = &["interface", "enum", "struct", "record", "trait", "object"];

/// Keywords that create node kind + add themselves as modifier (shapes)
pub const NODE_KEYWORDS: &[&str] = &["rectangle", "circle", "diamond"];

/// Check if a string is a known node kind
pub fn is_node_kind(s: &str) -> bool {
    KNOWN_NODE_KINDS.contains(&s)
}

/// Check if a string is a class-mapped keyword (returns the keyword as modifier)
pub fn class_keyword(s: &str) -> Option<&'static str> {
    CLASS_KEYWORDS.iter().find(|&&kw| kw == s).copied()
}

/// Check if a string is a node-mapped keyword (returns the shape as modifier)
pub fn node_keyword(s: &str) -> Option<&'static str> {
    NODE_KEYWORDS.iter().find(|&&kw| kw == s).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arrow_registry_has_both_directions() {
        let registry = get_arrow_registry();
        
        // Check that --> and <-- both exist
        assert!(registry.iter().any(|e| e.token == "-->"));
        assert!(registry.iter().any(|e| e.token == "<--"));
        
        // Check canonical names
        let right = registry.iter().find(|e| e.token == "-->").unwrap();
        assert_eq!(right.canonical_name, "assoc_right");
        
        let left = registry.iter().find(|e| e.token == "<--").unwrap();
        assert_eq!(left.canonical_name, "assoc_left");
    }

    #[test]
    fn test_arrow_registry_sorted_by_length() {
        let registry = get_arrow_registry();
        
        // Verify tokens are sorted by length (longest first)
        for i in 1..registry.len() {
            assert!(
                registry[i - 1].token.len() >= registry[i].token.len(),
                "Registry not sorted: {} before {}",
                registry[i - 1].token,
                registry[i].token
            );
        }
    }

    #[test]
    fn test_non_directional_arrows() {
        let registry = get_arrow_registry();
        
        // Line and dotted should not have direction suffixes
        let line = registry.iter().find(|e| e.token == "---").unwrap();
        assert_eq!(line.canonical_name, "line");
        
        let dotted = registry.iter().find(|e| e.token == "..").unwrap();
        assert_eq!(dotted.canonical_name, "dotted");
    }

    #[test]
    fn test_arrow_from_token() {
        assert_eq!(arrow_from_token("-->"), Some("assoc_right"));
        assert_eq!(arrow_from_token("<--"), Some("assoc_left"));
        assert_eq!(arrow_from_token("---|>"), None);
    }

    #[test]
    fn test_token_from_arrow() {
        assert_eq!(token_from_arrow("assoc_right"), Some("-->"));
        assert_eq!(token_from_arrow("assoc_left"), Some("<--"));
        assert_eq!(token_from_arrow("line"), Some("---"));
    }

    #[test]
    fn test_extends_arrows() {
        let registry = get_arrow_registry();
        
        // --|> should exist
        let extends_right = registry.iter().find(|e| e.token == "--|>").unwrap();
        assert_eq!(extends_right.canonical_name, "extends_right");
        
        // <|-- should be auto-generated
        let extends_left = registry.iter().find(|e| e.token == "<|--").unwrap();
        assert_eq!(extends_left.canonical_name, "extends_left");
    }

    #[test]
    fn test_implements_arrows() {
        let registry = get_arrow_registry();
        
        // ..|> should exist
        let impl_right = registry.iter().find(|e| e.token == "..|>").unwrap();
        assert_eq!(impl_right.canonical_name, "implements_right");
        assert_eq!(impl_right.definition.line_style, LineStyle::Dashed);
        
        // <|.. should be auto-generated
        let impl_left = registry.iter().find(|e| e.token == "<|..").unwrap();
        assert_eq!(impl_left.canonical_name, "implements_left");
    }
}
