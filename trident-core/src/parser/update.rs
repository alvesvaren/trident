//! Update positions in the AST for drag operations.
//!
//! This module provides functions to update the position of classes and groups
//! in the AST, which can then be emitted back to source code.

use crate::parser::types::*;

/// Update the position of a class node by ID.
/// Returns true if the class was found and updated.
pub fn update_class_position(ast: &mut FileAst, class_id: &str, new_pos: PointI) -> bool {
    find_and_update_class(&mut ast.items, class_id, new_pos)
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

/// Recursively search for a class by ID and update its position
fn find_and_update_class(items: &mut [Stmt], class_id: &str, new_pos: PointI) -> bool {
    for stmt in items {
        match stmt {
            Stmt::Class(c) if c.id.0 == class_id => {
                c.pos = Some(new_pos);
                return true;
            }
            Stmt::Group(g) => {
                if find_and_update_class(&mut g.items, class_id, new_pos) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse_file, emit_file};

    #[test]
    fn test_update_class_position() {
        let input = "class Foo\n";
        let mut ast = parse_file(input).unwrap();
        
        let updated = update_class_position(&mut ast, "Foo", PointI { x: 100, y: 200 });
        assert!(updated);
        
        let output = emit_file(&ast);
        assert!(output.contains("@pos: (100, 200)"));
    }

    #[test]
    fn test_update_class_position_existing() {
        let input = "class Foo {\n    @pos: (10, 20)\n}\n";
        let mut ast = parse_file(input).unwrap();
        
        let updated = update_class_position(&mut ast, "Foo", PointI { x: 100, y: 200 });
        assert!(updated);
        
        let output = emit_file(&ast);
        assert!(output.contains("@pos: (100, 200)"));
        assert!(!output.contains("@pos: (10, 20)"));
    }

    #[test]
    fn test_update_class_in_group() {
        let input = "group MyGroup {\n    class Foo\n}\n";
        let mut ast = parse_file(input).unwrap();
        
        let updated = update_class_position(&mut ast, "Foo", PointI { x: 50, y: 60 });
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
    fn test_update_nonexistent_class() {
        let input = "class Foo\n";
        let mut ast = parse_file(input).unwrap();
        
        let updated = update_class_position(&mut ast, "Bar", PointI { x: 100, y: 200 });
        assert!(!updated);
    }
}
