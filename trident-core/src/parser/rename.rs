//! Symbol renaming support for the Trident language.
//!
//! Provides functions to find all references to a symbol and rename them.

use crate::parser::{FileAst, GroupAst, NodeAst, RelationAst, Stmt, Ident};

/// A reference to a symbol in the source code.
#[derive(Debug, Clone)]
pub struct SymbolReference {
    /// 1-based line number
    pub line: usize,
    /// 1-based start column
    pub start_col: usize,
    /// 1-based end column (exclusive)
    pub end_col: usize,
    /// The symbol text
    pub text: String,
}

/// Find all references to a symbol (node ID or group ID) in the AST.
pub fn find_symbol_references(ast: &FileAst, symbol: &str) -> Vec<SymbolReference> {
    let mut refs = Vec::new();
    find_in_items(&ast.items, symbol, &mut refs);
    refs
}

fn find_in_items(items: &[Stmt], symbol: &str, refs: &mut Vec<SymbolReference>) {
    for stmt in items {
        match stmt {
            Stmt::Node(node) => find_in_node(node, symbol, refs),
            Stmt::Group(group) => find_in_group(group, symbol, refs),
            Stmt::Relation(rel) => find_in_relation(rel, symbol, refs),
            Stmt::Comment(_) => {}
        }
    }
}

fn find_in_node(node: &NodeAst, symbol: &str, refs: &mut Vec<SymbolReference>) {
    if node.id.0 == symbol {
        if let Some(span) = &node.span {
            // The node ID appears on the first line of the span
            // We need to find the column position - it's after `kind` and any modifiers
            refs.push(SymbolReference {
                line: span.start_line,
                start_col: 0, // Will be computed during rename
                end_col: 0,
                text: symbol.to_string(),
            });
        }
    }
}

fn find_in_group(group: &GroupAst, symbol: &str, refs: &mut Vec<SymbolReference>) {
    // Check group ID
    if let Some(id) = &group.id {
        if id.0 == symbol {
            if let Some(span) = &group.span {
                refs.push(SymbolReference {
                    line: span.start_line,
                    start_col: 0,
                    end_col: 0,
                    text: symbol.to_string(),
                });
            }
        }
    }
    // Recurse into group items
    find_in_items(&group.items, symbol, refs);
}

fn find_in_relation(rel: &RelationAst, symbol: &str, refs: &mut Vec<SymbolReference>) {
    if let Some(span) = &rel.span {
        if rel.from.0 == symbol {
            refs.push(SymbolReference {
                line: span.start_line,
                start_col: 0,
                end_col: 0,
                text: symbol.to_string(),
            });
        }
        if rel.to.0 == symbol {
            refs.push(SymbolReference {
                line: span.start_line,
                start_col: 0,
                end_col: 0,
                text: symbol.to_string(),
            });
        }
    }
}

/// Rename a symbol in the AST, returning the modified AST.
pub fn rename_symbol_in_ast(ast: &mut FileAst, old_name: &str, new_name: &str) -> bool {
    rename_in_items(&mut ast.items, old_name, new_name)
}

fn rename_in_items(items: &mut [Stmt], old_name: &str, new_name: &str) -> bool {
    let mut found = false;
    for stmt in items.iter_mut() {
        match stmt {
            Stmt::Node(node) => {
                if rename_in_node(node, old_name, new_name) {
                    found = true;
                }
            }
            Stmt::Group(group) => {
                if rename_in_group(group, old_name, new_name) {
                    found = true;
                }
            }
            Stmt::Relation(rel) => {
                if rename_in_relation(rel, old_name, new_name) {
                    found = true;
                }
            }
            Stmt::Comment(_) => {}
        }
    }
    found
}

fn rename_in_node(node: &mut NodeAst, old_name: &str, new_name: &str) -> bool {
    if node.id.0 == old_name {
        node.id = Ident(new_name.to_string());
        true
    } else {
        false
    }
}

fn rename_in_group(group: &mut GroupAst, old_name: &str, new_name: &str) -> bool {
    let mut found = false;
    
    // Rename group ID if it matches
    if let Some(id) = &group.id {
        if id.0 == old_name {
            group.id = Some(Ident(new_name.to_string()));
            found = true;
        }
    }
    
    // Recurse into items
    if rename_in_items(&mut group.items, old_name, new_name) {
        found = true;
    }
    
    found
}

fn rename_in_relation(rel: &mut RelationAst, old_name: &str, new_name: &str) -> bool {
    let mut found = false;
    
    if rel.from.0 == old_name {
        rel.from = Ident(new_name.to_string());
        found = true;
    }
    
    if rel.to.0 == old_name {
        rel.to = Ident(new_name.to_string());
        found = true;
    }
    
    found
}

/// Collect all defined symbol names (nodes and groups) from the AST.
pub fn collect_symbols(ast: &FileAst) -> Vec<String> {
    let mut symbols = Vec::new();
    collect_from_items(&ast.items, &mut symbols);
    symbols
}

fn collect_from_items(items: &[Stmt], symbols: &mut Vec<String>) {
    for stmt in items {
        match stmt {
            Stmt::Node(node) => {
                symbols.push(node.id.0.clone());
            }
            Stmt::Group(group) => {
                if let Some(id) = &group.id {
                    symbols.push(id.0.clone());
                }
                collect_from_items(&group.items, symbols);
            }
            Stmt::Relation(_) | Stmt::Comment(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_file;
    
    #[test]
    fn test_collect_symbols() {
        let input = "class Foo\nclass Bar\nFoo --> Bar\n";
        let ast = parse_file(input).unwrap();
        let symbols = collect_symbols(&ast);
        assert!(symbols.contains(&"Foo".to_string()));
        assert!(symbols.contains(&"Bar".to_string()));
        assert_eq!(symbols.len(), 2);
    }
    
    #[test]
    fn test_rename_symbol() {
        let input = "class Foo\nclass Bar\nFoo --> Bar\n";
        let mut ast = parse_file(input).unwrap();
        let found = rename_symbol_in_ast(&mut ast, "Foo", "Baz");
        assert!(found);
        
        // Check node was renamed
        match &ast.items[0] {
            Stmt::Node(n) => assert_eq!(n.id.0, "Baz"),
            _ => panic!("Expected node"),
        }
        
        // Check relation was renamed
        match &ast.items[2] {
            Stmt::Relation(r) => assert_eq!(r.from.0, "Baz"),
            _ => panic!("Expected relation"),
        }
    }
    
    #[test]
    fn test_rename_in_group() {
        let input = "group MyGroup {\n  class Foo\n}\nFoo --> Foo\n";
        let mut ast = parse_file(input).unwrap();
        
        // Rename the node inside the group
        let found = rename_symbol_in_ast(&mut ast, "Foo", "Bar");
        assert!(found);
        
        // Check the relation was updated
        match &ast.items[1] {
            Stmt::Relation(r) => {
                assert_eq!(r.from.0, "Bar");
                assert_eq!(r.to.0, "Bar");
            }
            _ => panic!("Expected relation"),
        }
    }
}
