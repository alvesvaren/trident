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
    /// Node kind: "class", "interface", "enum", "struct", etc.
    pub kind: String,
    /// Modifiers: "abstract", "static", "sealed", etc.
    pub modifiers: Vec<String>,
    /// Unique identifier
    pub id: Ident,
    /// Display label (optional)
    pub label: Option<String>,
    /// local position relative to closest parent group (or root)
    pub pos: Option<PointI>,
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

/// Known node kinds that the parser recognizes
pub const KNOWN_NODE_KINDS: &[&str] = &[
    "class",
    "interface",
    "enum",
    "struct",
    "record",
    "trait",
    "object",
];

/// Check if a string is a known node kind
pub fn is_node_kind(s: &str) -> bool {
    KNOWN_NODE_KINDS.contains(&s)
}