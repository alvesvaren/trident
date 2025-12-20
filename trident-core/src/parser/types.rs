use serde::{Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct FileAst {
    pub items: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize)]
pub enum Stmt {
    Group(GroupAst),
    Class(ClassAst),
    Relation(RelationAst),
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
}

#[derive(Debug, Clone, Serialize)]
pub struct ClassAst {
    pub id: Ident,
    pub label: Option<String>,
    /// local position relative to closest parent group (or root)
    pub pos: Option<PointI>,
    /// opaque lines inside class block (renderer decides)
    pub body_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelationAst {
    pub from: Ident,
    pub arrow: Arrow,
    pub to: Ident,
    pub label: Option<String>,
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