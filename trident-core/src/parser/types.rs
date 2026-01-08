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
// Arrow Registry - Single source of truth for arrow tokens
// ============================================================================

/// Arrow token registry: (token_str, canonical_name)
/// Longer tokens must come first to avoid partial matches
pub const ARROW_REGISTRY: &[(&str, &str)] = &[
    ("<|..", "dep_extends_left"),
    ("..|>", "dep_extends_right"),
    ("<|--", "extends_left"),
    ("--|>", "extends_right"),
    ("..>", "dep_right"),
    ("<..", "dep_left"),
    ("---", "line"),
    ("-->", "assoc_right"),
    ("<--", "assoc_left"),
    ("o--", "aggregate"),
    ("*--", "compose"),
    ("..", "dotted"),
];

/// Look up canonical name from token string
pub fn arrow_from_token(token: &str) -> Option<&'static str> {
    ARROW_REGISTRY
        .iter()
        .find(|(tok, _)| *tok == token)
        .map(|(_, name)| *name)
}

/// Look up token string from canonical name
pub fn token_from_arrow(arrow: &str) -> Option<&'static str> {
    ARROW_REGISTRY
        .iter()
        .find(|(_, name)| *name == arrow)
        .map(|(tok, _)| *tok)
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