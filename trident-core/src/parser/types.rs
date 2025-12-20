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
    Class(ClassAst),
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

#[derive(Debug, Clone, Serialize)]
pub struct ClassAst {
    pub id: Ident,
    pub label: Option<String>,
    /// local position relative to closest parent group (or root)
    pub pos: Option<PointI>,
    /// opaque lines inside class block (renderer decides)
    pub body_lines: Vec<String>,
    /// Source span for round-tripping
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelationAst {
    pub from: Ident,
    pub arrow: Arrow,
    pub to: Ident,
    pub label: Option<String>,
    /// Source span for round-tripping
    pub span: Option<Span>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Arrow {
    ExtendsLeft,  // <|--
    ExtendsRight, // --|>
    Aggregate,    // o--
    Compose,      // *--
    AssocRight,   // -->
    AssocLeft,    // <--
    DepRight,     // ..>
    DepLeft,      // <..
    Line,         // ---
    Dotted,       // ..
}