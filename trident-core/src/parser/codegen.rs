//! Emit AST back to source code, preserving structure and comments.
//!
//! Formatting rules:
//! - 4 spaces for indentation
//! - Always wrap brackets on their own lines for groups/classes
//! - Comments are preserved exactly as-is

use crate::parser::types::*;

const INDENT: &str = "    "; // 4 spaces

/// Emit the entire file AST back to source code
pub fn emit_file(ast: &FileAst) -> String {
    let mut out = String::new();
    for stmt in &ast.items {
        emit_stmt(stmt, 0, &mut out);
    }
    out
}

/// Emit a single statement with the given indentation level
fn emit_stmt(stmt: &Stmt, indent: usize, out: &mut String) {
    match stmt {
        Stmt::Group(g) => emit_group(g, indent, out),
        Stmt::Class(c) => emit_class(c, indent, out),
        Stmt::Relation(r) => emit_relation(r, indent, out),
        Stmt::Comment(c) => emit_comment(c, out),
    }
}

/// Emit a comment, preserving exact prefix and text
fn emit_comment(c: &CommentAst, out: &mut String) {
    // Empty comments (blank lines) have empty text
    if c.text.is_empty() && !c.prefix.contains("%%") {
        // This is a preserved blank line
        out.push_str(&c.prefix);
        out.push('\n');
    } else {
        // Regular comment
        out.push_str(&c.prefix);
        out.push_str("%%");
        out.push_str(&c.text);
        out.push('\n');
    }
}

/// Generate the indent string for a given level
fn indent_str(level: usize) -> String {
    INDENT.repeat(level)
}

/// Emit a group definition
fn emit_group(g: &GroupAst, indent: usize, out: &mut String) {
    let ind = indent_str(indent);
    
    // Group header
    if let Some(id) = &g.id {
        out.push_str(&format!("{}group {}\n", ind, id.0));
    } else {
        out.push_str(&format!("{}group\n", ind));
    }
    
    // Opening brace
    out.push_str(&format!("{}{{\n", ind));
    
    // @pos if present
    if let Some(pos) = &g.pos {
        emit_pos(pos, indent + 1, out);
    }
    
    // Items
    for stmt in &g.items {
        emit_stmt(stmt, indent + 1, out);
    }
    
    // Closing brace
    out.push_str(&format!("{}}}\n", ind));
}

/// Emit a class definition
fn emit_class(c: &ClassAst, indent: usize, out: &mut String) {
    let ind = indent_str(indent);
    
    // Class header
    let mut header = format!("{}class {}", ind, c.id.0);
    
    // Label if present
    if let Some(label) = &c.label {
        header.push_str(&format!(" \"{}\"", label));
    }
    
    // If class has pos or body_lines, emit with block
    if c.pos.is_some() || !c.body_lines.is_empty() {
        out.push_str(&header);
        out.push('\n');
        out.push_str(&format!("{}{{\n", ind));
        
        // @pos if present
        if let Some(pos) = &c.pos {
            emit_pos(pos, indent + 1, out);
        }
        
        // Body lines
        for line in &c.body_lines {
            out.push_str(&format!("{}{}\n", indent_str(indent + 1), line));
        }
        
        out.push_str(&format!("{}}}\n", ind));
    } else {
        // Simple class without block
        out.push_str(&header);
        out.push('\n');
    }
}

/// Emit a relation
fn emit_relation(r: &RelationAst, indent: usize, out: &mut String) {
    let ind = indent_str(indent);
    let arrow_str = arrow_to_str(r.arrow);
    
    let mut line = format!("{}{} {} {}", ind, r.from.0, arrow_str, r.to.0);
    
    if let Some(label) = &r.label {
        line.push_str(&format!(" : {}", label));
    }
    
    out.push_str(&line);
    out.push('\n');
}

/// Emit a @pos line
fn emit_pos(pos: &PointI, indent: usize, out: &mut String) {
    let ind = indent_str(indent);
    out.push_str(&format!("{}@pos: ({}, {})\n", ind, pos.x, pos.y));
}

/// Convert Arrow enum to its string representation
fn arrow_to_str(arrow: Arrow) -> &'static str {
    match arrow {
        Arrow::ExtendsLeft => "<|--",
        Arrow::ExtendsRight => "--|>",
        Arrow::DepRight => "..>",
        Arrow::DepLeft => "<..",
        Arrow::Line => "---",
        Arrow::AssocRight => "-->",
        Arrow::AssocLeft => "<--",
        Arrow::Aggregate => "o--",
        Arrow::Compose => "*--",
        Arrow::Dotted => "..",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_file;

    #[test]
    fn test_roundtrip_simple_class() {
        let input = "class Foo\n";
        let ast = parse_file(input).unwrap();
        let output = emit_file(&ast);
        assert_eq!(output.trim(), "class Foo");
    }

    #[test]
    fn test_roundtrip_class_with_label() {
        let input = "class Foo \"My Label\"\n";
        let ast = parse_file(input).unwrap();
        let output = emit_file(&ast);
        assert_eq!(output.trim(), "class Foo \"My Label\"");
    }

    #[test]
    fn test_roundtrip_class_with_pos() {
        let input = "class Foo {\n    @pos: (10, 20)\n}\n";
        let ast = parse_file(input).unwrap();
        let output = emit_file(&ast);
        assert!(output.contains("class Foo"));
        assert!(output.contains("@pos: (10, 20)"));
    }

    #[test]
    fn test_roundtrip_relation() {
        let input = "class A\nclass B\nA --> B\n";
        let ast = parse_file(input).unwrap();
        let output = emit_file(&ast);
        assert!(output.contains("A --> B"));
    }

    #[test]
    fn test_roundtrip_with_comment() {
        let input = "%% This is a comment\nclass Foo\n";
        let ast = parse_file(input).unwrap();
        let output = emit_file(&ast);
        assert!(output.contains("%% This is a comment"));
        assert!(output.contains("class Foo"));
    }

    #[test]
    fn test_roundtrip_group() {
        let input = "group MyGroup\n{\n    class Foo\n}\n";
        let ast = parse_file(input).unwrap();
        let output = emit_file(&ast);
        assert!(output.contains("group MyGroup"));
        assert!(output.contains("class Foo"));
    }
}
