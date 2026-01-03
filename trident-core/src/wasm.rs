//! WASM bindings for the trident-core library.
//!
//! All functions exposed to JavaScript via wasm-bindgen are defined here.

use wasm_bindgen::prelude::*;
use serde_json::to_string;

use crate::layout::{layout_diagram, LayoutConfig, RectI};
use crate::output::{DiagramOutput, NodeOutput, EdgeOutput, GroupOutput, ErrorInfo};
use crate::parser::{self, PointI};

#[wasm_bindgen]
extern "C" {
    pub fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn console_log(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = error)]
    pub fn console_error(s: &str);
}

#[wasm_bindgen]
pub fn compile_diagram(input: &str) -> String {
    let ast = match parser::parse_file(input) {
        Ok(ast) => ast,
        Err(e) => {
            console_error(&format!("Error parsing file: {:?}", e));
            let error_output = DiagramOutput {
                groups: vec![],
                nodes: vec![],
                edges: vec![],
                implicit_nodes: vec![],
                error: Some(ErrorInfo {
                    message: e.msg.clone(),
                    line: e.line,
                    column: e.col,
                    end_line: e.line,
                    end_column: e.col + 1, // Highlight at least one character
                }),
            };
            return serde_json::to_string(&error_output).unwrap();
        }
    };
    let diagram = match parser::compile(&ast) {
        Ok(diagram) => diagram,
        Err(e) => {
            console_error(&format!("Error compiling file: {:?}", e));
            let error_output = DiagramOutput {
                groups: vec![],
                nodes: vec![],
                edges: vec![],
                implicit_nodes: vec![],
                error: Some(ErrorInfo {
                    message: e.msg.clone(),
                    line: e.line,
                    column: e.col,
                    end_line: e.line,
                    end_column: 1000, // Highlight the whole line
                }),
            };
            return serde_json::to_string(&error_output).unwrap();
        }
    };
    
    // Use the layout algorithm specified in the AST, or default to hierarchical
    let layout_name = ast.layout.as_deref().unwrap_or("hierarchical");
    let layout_result = layout_diagram(&diagram, &LayoutConfig::default(), layout_name);
    
    // Build groups (only named groups, skip root and anonymous)
    let groups: Vec<GroupOutput> = diagram.groups.iter()
        .filter(|g| g.id.is_some() && g.gid != diagram.root)
        .filter_map(|g| {
            let bounds = layout_result.group_world_bounds.get(&g.gid).copied()?;
            Some(GroupOutput {
                id: g.id.as_ref()?.0.clone(),
                bounds,
            })
        })
        .collect();
    
    // Build nodes
    let nodes: Vec<NodeOutput> = diagram.nodes.iter().map(|n| {
        let bounds = layout_result.node_world_bounds.get(&n.nid).copied().unwrap_or(RectI { x: 0, y: 0, w: 0, h: 0 });
        // Get parent group's world position for local coordinate calculation
        let parent_world = layout_result.group_world_pos.get(&n.group).copied().unwrap_or(PointI { x: 0, y: 0 });
        NodeOutput {
            id: n.id.0.clone(),
            kind: n.kind.clone(),
            modifiers: n.modifiers.clone(),
            label: n.label.clone(),
            body_lines: n.body_lines.clone(),
            bounds,
            has_pos: n.pos.is_some(),
            parent_offset: parent_world,
            explicit: n.explicit,
        }
    }).collect();
    
    // Build edges
    let edges: Vec<EdgeOutput> = diagram.edges.iter().map(|e| {
        let from_id = diagram.nodes[e.from.0].id.0.clone();
        let to_id = diagram.nodes[e.to.0].id.0.clone();
        EdgeOutput {
            from: from_id,
            to: to_id,
            arrow: e.arrow.clone(),
            label: e.label.clone(),
        }
    }).collect();
    
    // Collect implicit node IDs for editor diagnostics
    let implicit_nodes: Vec<String> = diagram.nodes.iter()
        .filter(|n| !n.explicit)
        .map(|n| n.id.0.clone())
        .collect();
    
    let output = DiagramOutput { groups, nodes, edges, implicit_nodes, error: None };
    to_string(&output).unwrap()
}

/// Update a node position and return the new source code
#[wasm_bindgen]
pub fn update_class_pos(source: &str, class_id: &str, x: i32, y: i32) -> String {
    let mut ast = match parser::parse_file(source) {
        Ok(ast) => ast,
        Err(e) => {
            console_error(&format!("Error parsing file: {:?}", e));
            return source.to_string();
        }
    };
    
    let new_pos = PointI { x, y };
    if parser::update_node_position(&mut ast, class_id, new_pos) {
        parser::emit_file(&ast)
    } else {
        console_error(&format!("Node '{}' not found", class_id));
        source.to_string()
    }
}

/// Update a group position and return the new source code.
/// For named groups: pass the group_id.
/// For anonymous groups: pass empty string for group_id and use the group_index.
#[wasm_bindgen]
pub fn update_group_pos(source: &str, group_id: &str, group_index: usize, x: i32, y: i32) -> String {
    let mut ast = match parser::parse_file(source) {
        Ok(ast) => ast,
        Err(e) => {
            console_error(&format!("Error parsing file: {:?}", e));
            return source.to_string();
        }
    };
    
    let new_pos = PointI { x, y };
    let group_id_opt = if group_id.is_empty() { None } else { Some(group_id) };
    
    if parser::update_group_position(&mut ast, group_id_opt, group_index, new_pos) {
        parser::emit_file(&ast)
    } else {
        console_error(&format!("Group not found (id={:?}, index={})", group_id_opt, group_index));
        source.to_string()
    }
}

/// Remove a node position (unlock it for auto-layout) and return the new source code
#[wasm_bindgen]
pub fn remove_class_pos(source: &str, class_id: &str) -> String {
    let mut ast = match parser::parse_file(source) {
        Ok(ast) => ast,
        Err(e) => {
            console_error(&format!("Error parsing file: {:?}", e));
            return source.to_string();
        }
    };
    
    if parser::remove_node_position(&mut ast, class_id) {
        parser::emit_file(&ast)
    } else {
        console_error(&format!("Node '{}' not found", class_id));
        source.to_string()
    }
}

/// Remove all positions from all nodes and groups (unlock everything)
#[wasm_bindgen]
pub fn remove_all_pos(source: &str) -> String {
    let mut ast = match parser::parse_file(source) {
        Ok(ast) => ast,
        Err(e) => {
            console_error(&format!("Error parsing file: {:?}", e));
            return source.to_string();
        }
    };
    
    parser::remove_all_positions(&mut ast);
    parser::emit_file(&ast)
}

/// Insert a node declaration for an implicit node (created from a relation).
/// This is used when starting to drag an implicit node to make it explicit.
/// Returns the updated source code.
#[wasm_bindgen]
pub fn insert_implicit_node(source: &str, node_id: &str, x: i32, y: i32) -> String {
    let mut ast = match parser::parse_file(source) {
        Ok(ast) => ast,
        Err(e) => {
            console_error(&format!("Error parsing file: {:?}", e));
            return source.to_string();
        }
    };
    
    let pos = PointI { x, y };
    if parser::insert_implicit_node(&mut ast, node_id, pos) {
        parser::emit_file(&ast)
    } else {
        // Node already exists, nothing to do
        source.to_string()
    }
}

/// Rename a symbol (node ID or group ID) and return the updated source code.
/// Returns the original source if the symbol is not found or parsing fails.
#[wasm_bindgen]
pub fn rename_symbol(source: &str, old_name: &str, new_name: &str) -> String {
    let mut ast = match parser::parse_file(source) {
        Ok(ast) => ast,
        Err(e) => {
            console_error(&format!("Error parsing file: {:?}", e));
            return source.to_string();
        }
    };
    
    if parser::rename_symbol_in_ast(&mut ast, old_name, new_name) {
        parser::emit_file(&ast)
    } else {
        console_error(&format!("Symbol '{}' not found", old_name));
        source.to_string()
    }
}

/// Get all defined symbols (node IDs and group IDs) in the source.
/// Returns a JSON array of strings.
/// NOTE: This tries to parse the source and extract symbols even if there are errors.
#[wasm_bindgen]
pub fn get_symbols(source: &str) -> String {
    // Try parsing - if it fails, try a line-by-line fallback
    match parser::parse_file(source) {
        Ok(ast) => {
            let symbols = parser::collect_symbols(&ast);
            serde_json::to_string(&symbols).unwrap_or_else(|_| "[]".to_string())
        }
        Err(_) => {
            // Fallback: extract identifiers from node/class declarations using regex-like matching
            // This is a simple heuristic to get symbols even when parse fails
            let mut symbols: Vec<String> = Vec::new();
            for line in source.lines() {
                let trimmed = line.trim();
                // Skip comments and empty lines
                if trimmed.is_empty() || trimmed.starts_with("%%") {
                    continue;
                }
                // Look for node declarations: [modifiers] <kind> <identifier>
                let words: Vec<&str> = trimmed.split_whitespace().collect();
                if words.len() >= 2 {
                    // Check if any word is a node kind keyword
                    let kinds = ["class", "interface", "enum", "struct", "record", "trait", 
                                 "object", "node", "rectangle", "circle", "diamond"];
                    for (i, word) in words.iter().enumerate() {
                        if kinds.contains(word) && i + 1 < words.len() {
                            // Next word is the identifier (strip any trailing characters)
                            let id = words[i + 1]
                                .chars()
                                .take_while(|c| c.is_alphanumeric() || *c == '_')
                                .collect::<String>();
                            if !id.is_empty() && !symbols.contains(&id) {
                                symbols.push(id);
                            }
                            break;
                        }
                    }
                    // Check for group: group <identifier> { or group {
                    if words[0] == "group" && words.len() >= 2 {
                        let potential_id = words[1];
                        if potential_id != "{" {
                            let id = potential_id
                                .chars()
                                .take_while(|c| c.is_alphanumeric() || *c == '_')
                                .collect::<String>();
                            if !id.is_empty() && !symbols.contains(&id) {
                                symbols.push(id);
                            }
                        }
                    }
                }
            }
            serde_json::to_string(&symbols).unwrap_or_else(|_| "[]".to_string())
        }
    }
}

