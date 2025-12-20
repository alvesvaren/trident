// sdd_parser.rs
//
// v0.0.1 parser (cleaned):
// - Comments: %% ... (line comments)
// - group { ... }              (anonymous, not rendered; layout scope)
// - group IDENT { ... }        (named; visual name = IDENT)
// - class IDENT ["Display"]    (optionally with block)
// - class IDENT ["Display"] { ... }
// - @pos: (INT, INT) allowed only inside the nearest class/group block (fixed, local)
// - relations can be written with or without spaces:
//     A-->B
//     A --> B
//     A<|--B : label
//
// Notes / limitations (intentional for v0.0.1):
// - IDENT: [A-Za-z_][A-Za-z0-9_]*
// - STRING: "..." (no escapes)
// - Only one @pos per class/group block (duplicate is error)
// - Relation endpoints must be IDENT (no qualification yet)

use crate::parser::types::*;
use crate::parser::types::{CommentAst, Span};
use std::fmt;

impl Arrow {
    pub const TOKENS: &'static [(&'static str, Arrow)] = &[
        // IMPORTANT: longer tokens first to avoid partial matches
        ("<|--", Arrow::ExtendsLeft),
        ("--|>", Arrow::ExtendsRight),
        ("..>", Arrow::DepRight),
        ("<..", Arrow::DepLeft),
        ("---", Arrow::Line),
        ("-->", Arrow::AssocRight),
        ("<--", Arrow::AssocLeft),
        ("o--", Arrow::Aggregate),
        ("*--", Arrow::Compose),
        ("..", Arrow::Dotted),
    ];
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: usize, // 1-based
    pub col: usize,  // 1-based best-effort
    pub msg: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse error at {}:{}: {}", self.line, self.col, self.msg)
    }
}
impl std::error::Error for ParseError {}

pub fn parse_file(input: &str) -> Result<FileAst, ParseError> {
    let mut p = Parser::new(input);
    let items = p.parse_items_until_end()?;
    Ok(FileAst { items })
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let lines = input
            .lines()
            .map(|l| l.strip_suffix('\r').unwrap_or(l))
            .collect::<Vec<_>>();
        Self { lines, i: 0 }
    }

    fn eof(&self) -> bool {
        self.i >= self.lines.len()
    }

    fn line_no(&self) -> usize {
        self.i + 1
    }

    fn advance(&mut self) {
        self.i += 1;
    }

    fn err<T>(&self, col: usize, msg: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError {
            line: self.line_no(),
            col: col.max(1),
            msg: msg.into(),
        })
    }

    fn current_line_wo_comment(&self) -> &'a str {
        let raw = self.lines[self.i];
        match raw.find("%%") {
            Some(idx) => &raw[..idx],
            None => raw,
        }
    }

    /// Get the raw line (including comment)
    fn current_raw_line(&self) -> &'a str {
        self.lines[self.i]
    }

    /// Check if line is only whitespace and/or a comment
    fn is_comment_or_empty_line(&self) -> bool {
        let raw = self.lines[self.i];
        let without_comment = match raw.find("%%") {
            Some(idx) => &raw[..idx],
            None => raw,
        };
        without_comment.trim().is_empty()
    }

    /// Parse a comment line into CommentAst
    fn parse_comment_line(&self) -> Option<CommentAst> {
        let raw = self.lines[self.i];
        if let Some(idx) = raw.find("%%") {
            Some(CommentAst {
                prefix: raw[..idx].to_string(),
                text: raw[idx + 2..].to_string(),
            })
        } else if raw.trim().is_empty() {
            // Empty/whitespace line - preserve as empty comment
            Some(CommentAst {
                prefix: raw.to_string(),
                text: String::new(),
            })
        } else {
            None
        }
    }

    fn parse_items_until_end(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut items = Vec::new();
        while !self.eof() {
            if let Some(stmt) = self.parse_stmt_or_none()? {
                items.push(stmt);
            }
        }
        Ok(items)
    }

    fn parse_items_until_rbrace(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut items = Vec::new();
        loop {
            if self.eof() {
                return self.err(1, "unexpected end of file; missing '}'");
            }

            let t = self.current_line_wo_comment().trim();
            
            if t == "}" {
                self.advance();
                break;
            }

            if let Some(stmt) = self.parse_stmt_or_none()? {
                items.push(stmt);
            }
        }
        Ok(items)
    }

    fn parse_stmt_or_none(&mut self) -> Result<Option<Stmt>, ParseError> {
        if self.eof() {
            return Ok(None);
        }

        let t = self.current_line_wo_comment().trim();
        
        // Check for comment-only or empty line first
        if self.is_comment_or_empty_line() {
            if let Some(comment) = self.parse_comment_line() {
                self.advance();
                return Ok(Some(Stmt::Comment(comment)));
            }
            self.advance();
            return Ok(None);
        }

        if t == "classDiagram" {
            self.advance();
            return Ok(None);
        }

        if t == "}" {
            return self.err(1, "unexpected '}'");
        }

        if starts_with_kw(t, "group") {
            let g = self.parse_group()?;
            return Ok(Some(Stmt::Group(g)));
        }

        if starts_with_kw(t, "class") {
            let c = self.parse_class()?;
            return Ok(Some(Stmt::Class(c)));
        }

        // Otherwise, relation
        let start_line = self.line_no();
        let rel = self.parse_relation_line(t).map_err(|mut e| {
            // keep current line number
            e.line = self.line_no();
            e
        })?;
        self.advance();
        Ok(Some(Stmt::Relation(RelationAst {
            span: Some(Span { start_line, end_line: start_line }),
            ..rel
        })))
    }

    // group { ... }
    // group IDENT { ... }
    // allow '{' on same line OR next non-empty line
    fn parse_group(&mut self) -> Result<GroupAst, ParseError> {
        let start_line = self.line_no();
        let t = self.current_line_wo_comment().trim();

        // parse header: "group" [IDENT]? ["{"]?
        let mut rest = t.strip_prefix("group").unwrap().trim();

        let mut id: Option<Ident> = None;
        let mut has_lbrace = false;

        if rest.starts_with('{') {
            has_lbrace = true;
            rest = rest[1..].trim();
            if !rest.is_empty() {
                return self.err(1, "unexpected tokens after '{' in group header");
            }
        } else if !rest.is_empty() {
            // expect IDENT or IDENT followed by '{'
            // Allow "group G{" or "group G {"
            let (ident_part, after_ident) = take_ident_prefix(rest);
            let Some(ident) = ident_part else {
                return self.err(1, "expected '{' or group identifier after 'group'");
            };
            id = Some(Ident(ident.to_string()));
            rest = after_ident.trim();

            if rest.starts_with('{') {
                has_lbrace = true;
                rest = rest[1..].trim();
            }

            if !rest.is_empty() {
                return self.err(1, "unexpected tokens in group header");
            }
        }

        self.advance(); // consume header line

        if !has_lbrace {
            self.consume_required_lbrace("group")?;
        }

        // parse body: allow @pos lines, comments, and nested statements
        let mut pos: Option<PointI> = None;
        let mut items: Vec<Stmt> = Vec::new();

        loop {
            if self.eof() {
                return self.err(1, "unexpected end of file; missing '}' for group");
            }

            let t = self.current_line_wo_comment().trim();
            
            if t == "}" {
                let end_line = self.line_no();
                self.advance();
                return Ok(GroupAst {
                    id,
                    pos,
                    items,
                    span: Some(Span { start_line, end_line }),
                });
            }

            if t.starts_with("@pos:") {
                if pos.is_some() {
                    return self.err(1, "duplicate @pos in group block");
                }
                pos = Some(parse_pos_line(t).map_err(|msg| ParseError {
                    line: self.line_no(),
                    col: 1,
                    msg,
                })?);
                self.advance();
                continue;
            }

            if let Some(stmt) = self.parse_stmt_or_none()? {
                items.push(stmt);
            }
        }
    }

    // class IDENT ["Label"] [ "{" ... "}" ]?
    // allow '{' on same line OR next non-empty line
    fn parse_class(&mut self) -> Result<ClassAst, ParseError> {
        let start_line = self.line_no();
        let t = self.current_line_wo_comment().trim();
        let mut rest = t.strip_prefix("class").unwrap().trim();

        let (ident_part, after_ident) = take_ident_prefix(rest);
        let Some(ident) = ident_part else {
            return self.err(1, "expected class identifier after 'class'");
        };
        if !is_ident(ident) {
            return self.err(1, "invalid class identifier");
        }
        let id = Ident(ident.to_string());
        rest = after_ident.trim();

        // optional label string
        let mut label: Option<String> = None;
        if rest.starts_with('"') {
            let (s, after) = parse_string_prefix(rest).map_err(|msg| ParseError {
                line: self.line_no(),
                col: 1,
                msg,
            })?;
            label = Some(s);
            rest = after.trim();
        }

        // optional '{' on same line
        let mut has_lbrace = false;
        if rest.starts_with('{') {
            has_lbrace = true;
            rest = rest[1..].trim();
        }
        if !rest.is_empty() {
            return self.err(1, "unexpected tokens in class declaration");
        }

        self.advance(); // consume class header

        // no block => empty class
        if !has_lbrace {
            // maybe next line is '{' to start block
            // If next meaningful line is '{', treat it as block start.
            if self.peek_next_nonempty_is_lbrace() {
                self.consume_required_lbrace("class")?;
                has_lbrace = true;
            }
        }

        if !has_lbrace {
            // Single-line class declaration, span is just the header line
            return Ok(ClassAst {
                id,
                label,
                pos: None,
                body_lines: Vec::new(),
                span: Some(Span { start_line, end_line: start_line }),
            });
        }

        let mut pos: Option<PointI> = None;
        let mut body_lines: Vec<String> = Vec::new();

        loop {
            if self.eof() {
                return self.err(1, "unexpected end of file; missing '}' for class");
            }

            let t = self.current_line_wo_comment().trim();
            if t.is_empty() {
                self.advance();
                continue;
            }
            if t == "}" {
                let end_line = self.line_no();
                self.advance();
                return Ok(ClassAst {
                    id,
                    label,
                    pos,
                    body_lines,
                    span: Some(Span { start_line, end_line }),
                });
            }

            if t.starts_with("@pos:") {
                if pos.is_some() {
                    return self.err(1, "duplicate @pos in class block");
                }
                pos = Some(parse_pos_line(t).map_err(|msg| ParseError {
                    line: self.line_no(),
                    col: 1,
                    msg,
                })?);
                self.advance();
                continue;
            }

            // opaque line
            body_lines.push(t.to_string());
            self.advance();
        }
    }

    fn parse_relation_line(&self, line: &str) -> Result<RelationAst, ParseError> {
        // Split label on first ':' (if any)
        let (head, label) = match line.split_once(':') {
            Some((a, b)) => {
                let l = b.trim();
                (
                    a.trim(),
                    if l.is_empty() {
                        None
                    } else {
                        Some(l.to_string())
                    },
                )
            }
            None => (line, None),
        };

        let (from, arrow, to) = split_relation_compact(head).ok_or_else(|| ParseError {
            line: self.line_no(),
            col: 1,
            msg: "invalid relation; expected like A-->B or A --> B".into(),
        })?;

        Ok(RelationAst {
            from: Ident(from.to_string()),
            arrow,
            to: Ident(to.to_string()),
            label,
            span: None, // Span is added by parse_stmt_or_none
        })
    }

    fn consume_required_lbrace(&mut self, ctx: &str) -> Result<(), ParseError> {
        while !self.eof() {
            let t = self.current_line_wo_comment().trim();
            if t.is_empty() {
                self.advance();
                continue;
            }
            if t == "{" {
                self.advance();
                return Ok(());
            }
            return self.err(1, format!("expected '{{' to start {ctx} block"));
        }
        self.err(1, "unexpected end of file while looking for '{'")
    }

    fn peek_next_nonempty_is_lbrace(&self) -> bool {
        let mut j = self.i;
        while j < self.lines.len() {
            let raw = self.lines[j];
            let wo = match raw.find("%%") {
                Some(idx) => &raw[..idx],
                None => raw,
            };
            let t = wo.trim();
            if t.is_empty() {
                j += 1;
                continue;
            }
            return t == "{";
        }
        false
    }
}

// ---------- helpers ----------

fn starts_with_kw(line: &str, kw: &str) -> bool {
    line == kw
        || line.starts_with(&(kw.to_string() + " "))
        || line.starts_with(&(kw.to_string() + "{"))
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn take_ident_prefix(s: &str) -> (Option<&str>, &str) {
    let s = s.trim_start();
    let mut end = 0usize;
    for (i, c) in s.char_indices() {
        if i == 0 {
            if !(c.is_ascii_alphabetic() || c == '_') {
                return (None, s);
            }
            end = c.len_utf8();
        } else {
            if c.is_ascii_alphanumeric() || c == '_' {
                end = i + c.len_utf8();
            } else {
                break;
            }
        }
    }
    if end == 0 {
        (None, s)
    } else {
        (Some(&s[..end]), &s[end..])
    }
}

fn parse_string_prefix(s: &str) -> Result<(String, &str), String> {
    let s = s.trim_start();
    if !s.starts_with('"') {
        return Err("expected string".into());
    }
    let mut chars = s.chars();
    chars.next(); // opening quote
    let mut out = String::new();
    while let Some(c) = chars.next() {
        if c == '"' {
            // idx points to byte offset? we track by slicing using char iteration; simplest:
            // find the closing quote in bytes:
            let close = s[1..]
                .find('"')
                .ok_or_else(|| "unterminated string".to_string())?
                + 1;
            let content = &s[1..close];
            let rest = &s[close + 1..];
            return Ok((content.to_string(), rest));
        }
        out.push(c);
    }
    Err("unterminated string literal".into())
}

fn parse_pos_line(t: &str) -> Result<PointI, String> {
    // @pos: (INT, INT)
    let rest = t
        .strip_prefix("@pos:")
        .ok_or_else(|| "expected '@pos:'".to_string())?;
    let rest = rest.trim();
    if !rest.starts_with('(') || !rest.ends_with(')') {
        return Err("expected @pos: (x, y)".into());
    }
    let inner = &rest[1..rest.len() - 1];
    let mut parts = inner.split(',').map(|p| p.trim());

    let x = parts
        .next()
        .ok_or_else(|| "missing x".to_string())?
        .parse::<i32>()
        .map_err(|_| "x must be an integer".to_string())?;
    let y = parts
        .next()
        .ok_or_else(|| "missing y".to_string())?
        .parse::<i32>()
        .map_err(|_| "y must be an integer".to_string())?;

    if parts.next().is_some() {
        return Err("too many components in @pos; expected (x, y)".into());
    }
    Ok(PointI { x, y })
}

/// Parses relations with or without spaces.
/// Accepts:
/// - "A-->B"
/// - "A --> B"
/// - "A<|--B"
/// - "A <|-- B"
fn split_relation_compact(s: &str) -> Option<(&str, Arrow, &str)> {
    let s = s.trim();

    // Fast path: try whitespace split into 3 parts
    {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() == 3 {
            let (a, op, b) = (parts[0], parts[1], parts[2]);
            if is_ident(a) && is_ident(b) {
                for (tok, arrow) in Arrow::TOKENS {
                    if *tok == op {
                        return Some((a, *arrow, b));
                    }
                }
            }
        }
    }

    // Compact path: find any arrow token inside the string
    for (tok, arrow) in Arrow::TOKENS {
        if let Some(pos) = s.find(tok) {
            let left = s[..pos].trim();
            let right = s[pos + tok.len()..].trim();

            if is_ident(left) && is_ident(right) {
                return Some((left, *arrow, right));
            }
        }
    }

    None
}
