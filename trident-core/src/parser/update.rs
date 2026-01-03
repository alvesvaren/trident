//! Update positions in the AST for drag operations.
//!
//! This module provides functions to update the position of nodes and groups
//! in the AST, which can then be emitted back to source code.

use crate::parser::types::*;

/// Update the position of a node by ID.
/// Returns true if the node was found and updated.
pub fn update_node_position(ast: &mut FileAst, node_id: &str, new_pos: PointI) -> bool {
    find_and_update_node(&mut ast.items, node_id, new_pos)
}

/// Update the position of a group.
/// For named groups: pass the group_id.
/// For anonymous groups: pass None for group_id and use the group_index.
/// Returns true if the group was found and updated.
pub fn update_group_position(
    ast: &mut FileAst,
    group_id: Option<&str>,
    group_index: usize,
    new_pos: PointI,
) -> bool {
    let mut current_index = 0;
    find_and_update_group(
        &mut ast.items,
        group_id,
        group_index,
        &mut current_index,
        new_pos,
    )
}

/// Recursively search for a node by ID and update its position
fn find_and_update_node(items: &mut [Stmt], node_id: &str, new_pos: PointI) -> bool {
    for stmt in items {
        match stmt {
            Stmt::Node(n) if n.id.0 == node_id => {
                n.pos = Some(new_pos);
                return true;
            }
            Stmt::Group(g) => {
                if find_and_update_node(&mut g.items, node_id, new_pos) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Recursively search for a group and update its position.
/// For named groups, match by ID. For anonymous, match by traversal index.
fn find_and_update_group(
    items: &mut [Stmt],
    group_id: Option<&str>,
    target_index: usize,
    current_index: &mut usize,
    new_pos: PointI,
) -> bool {
    for stmt in items {
        if let Stmt::Group(g) = stmt {
            // Check if this is the target group
            let is_match = match (group_id, &g.id) {
                // Named group: match by ID
                (Some(target_id), Some(current_id)) => current_id.0 == target_id,
                // Anonymous group: match by index
                (None, None) => *current_index == target_index,
                // Named looking for anonymous or vice versa: no match
                _ => false,
            };

            if is_match {
                g.pos = Some(new_pos);
                return true;
            }

            *current_index += 1;

            // Recurse into children
            if find_and_update_group(&mut g.items, group_id, target_index, current_index, new_pos) {
                return true;
            }
        }
    }
    false
}

/// Remove the position of a node by ID (unlock it).
/// Returns true if the node was found and its position was removed.
pub fn remove_node_position(ast: &mut FileAst, node_id: &str) -> bool {
    find_and_remove_node_position(&mut ast.items, node_id)
}

/// Recursively search for a node by ID and remove its position
fn find_and_remove_node_position(items: &mut [Stmt], node_id: &str) -> bool {
    for stmt in items {
        match stmt {
            Stmt::Node(n) if n.id.0 == node_id => {
                n.pos = None;
                return true;
            }
            Stmt::Group(g) => {
                if find_and_remove_node_position(&mut g.items, node_id) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Remove all positions from all nodes and groups in the AST.
/// This "unlocks" everything for auto-layout.
pub fn remove_all_positions(ast: &mut FileAst) {
    remove_all_positions_recursive(&mut ast.items);
}

/// Recursively remove positions from all items
fn remove_all_positions_recursive(items: &mut [Stmt]) {
    for stmt in items {
        match stmt {
            Stmt::Node(n) => {
                n.pos = None;
            }
            Stmt::Group(g) => {
                g.pos = None;
                remove_all_positions_recursive(&mut g.items);
            }
            _ => {}
        }
    }
}

/// Insert a simple node declaration for an implicit node.
/// This is used when dragging an implicit node (created from a relation).
/// Returns true if the node was inserted (i.e., it didn't already exist).
pub fn insert_implicit_node(ast: &mut FileAst, node_id: &str, pos: PointI) -> bool {
    // First check if node already exists
    if node_exists(&ast.items, node_id) {
        return false;
    }
    
    // Create a simple node declaration
    let node = NodeAst {
        kind: "node".to_string(),
        original_kind: "node".to_string(),
        modifiers: vec!["rectangle".to_string()],
        id: Ident(node_id.to_string()),
        label: None,
        pos: Some(pos),
        width: None,
        height: None,
        body_lines: Vec::new(),
        span: None,
    };
    
    // Insert at the end of the file
    ast.items.push(Stmt::Node(node));
    true
}

/// Check if a node with the given ID exists in the AST
fn node_exists(items: &[Stmt], node_id: &str) -> bool {
    for stmt in items {
        match stmt {
            Stmt::Node(n) if n.id.0 == node_id => return true,
            Stmt::Group(g) => {
                if node_exists(&g.items, node_id) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse_file, emit_file};

    #[test]
    fn test_update_node_position() {
        let input = "class Foo\n";
        let mut ast = parse_file(input).unwrap();
        
        let updated = update_node_position(&mut ast, "Foo", PointI { x: 100, y: 200 });
        assert!(updated);
        
        let output = emit_file(&ast);
        assert!(output.contains("@pos: (100, 200)"));
    }

    #[test]
    fn test_update_node_position_existing() {
        let input = "class Foo {\n    @pos: (10, 20)\n}\n";
        let mut ast = parse_file(input).unwrap();
        
        let updated = update_node_position(&mut ast, "Foo", PointI { x: 100, y: 200 });
        assert!(updated);
        
        let output = emit_file(&ast);
        assert!(output.contains("@pos: (100, 200)"));
        assert!(!output.contains("@pos: (10, 20)"));
    }

    #[test]
    fn test_update_node_in_group() {
        let input = "group MyGroup {\n    class Foo\n}\n";
        let mut ast = parse_file(input).unwrap();
        
        let updated = update_node_position(&mut ast, "Foo", PointI { x: 50, y: 60 });
        assert!(updated);
        
        let output = emit_file(&ast);
        assert!(output.contains("@pos: (50, 60)"));
    }

    #[test]
    fn test_update_named_group_position() {
        let input = "group MyGroup {\n    class Foo\n}\n";
        let mut ast = parse_file(input).unwrap();
        
        let updated = update_group_position(&mut ast, Some("MyGroup"), 0, PointI { x: 100, y: 100 });
        assert!(updated);
        
        let output = emit_file(&ast);
        assert!(output.contains("@pos: (100, 100)"));
    }

    #[test]
    fn test_update_anonymous_group_position() {
        let input = "group {\n    class Foo\n}\n";
        let mut ast = parse_file(input).unwrap();
        
        let updated = update_group_position(&mut ast, None, 0, PointI { x: 50, y: 75 });
        assert!(updated);
        
        let output = emit_file(&ast);
        assert!(output.contains("@pos: (50, 75)"));
    }

    #[test]
    fn test_update_nonexistent_node() {
        let input = "class Foo\n";
        let mut ast = parse_file(input).unwrap();
        
        let updated = update_node_position(&mut ast, "Bar", PointI { x: 100, y: 200 });
        assert!(!updated);
    }

    #[test]
    fn test_update_interface_position() {
        let input = "interface IFoo\n";
        let mut ast = parse_file(input).unwrap();
        
        let updated = update_node_position(&mut ast, "IFoo", PointI { x: 100, y: 200 });
        assert!(updated);
        
        let output = emit_file(&ast);
        assert!(output.contains("interface IFoo"));
        assert!(output.contains("@pos: (100, 200)"));
    }

    #[test]
    fn test_update_preserves_other_positions() {
        // Two nodes, both with positions
        let input = "@layout: grid
class Foo {
    @pos: (10, 20)
}
class Bar {
    @pos: (100, 200)
}
";
        let mut ast = parse_file(input).unwrap();
        
        // Update Foo's position
        let updated = update_node_position(&mut ast, "Foo", PointI { x: 50, y: 60 });
        assert!(updated);
        
        let output = emit_file(&ast);
        println!("Output:\n{}", output);
        
        // Foo should have new position
        assert!(output.contains("@pos: (50, 60)"), "Foo's position should be updated");
        // Bar should keep its position
        assert!(output.contains("@pos: (100, 200)"), "Bar's position should be preserved");
    }
}
