use wasm_bindgen::prelude::*;
mod parser;
mod layout;
use layout::{layout_diagram, LayoutConfig, RectI};
use parser::PointI;
use serde::Serialize;
use serde_json::to_string;

#[wasm_bindgen]
extern "C" {
    pub fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn console_log(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = error)]
    pub fn console_error(s: &str);
}

/// A rendered node ready for React to display
#[derive(Debug, Clone, Serialize)]
pub struct NodeOutput {
    pub id: String,
    /// Node kind: "class", "interface", "enum", etc.
    pub kind: String,
    /// Modifiers: "abstract", "static", etc.
    pub modifiers: Vec<String>,
    pub label: Option<String>,
    pub body_lines: Vec<String>,
    pub bounds: RectI,
    /// Whether this node has a fixed position (@pos in the source)
    pub has_pos: bool,
    /// World position of parent group (for calculating local coords during drag)
    pub parent_offset: PointI,
}

/// An edge between two nodes
#[derive(Debug, Clone, Serialize)]
pub struct EdgeOutput {
    pub from: String,
    pub to: String,
    /// Arrow type as canonical string (e.g., "extends_left", "assoc_right")
    pub arrow: String,
    pub label: Option<String>,
}

/// A group container
#[derive(Debug, Clone, Serialize)]
pub struct GroupOutput {
    pub id: String,
    pub bounds: RectI,
}

/// The combined output sent to React
#[derive(Debug, Clone, Serialize)]
pub struct DiagramOutput {
    pub groups: Vec<GroupOutput>,
    pub nodes: Vec<NodeOutput>,
    pub edges: Vec<EdgeOutput>,
}

#[wasm_bindgen]
pub fn compile_diagram(input: &str) -> String {
    let ast = match parser::parse_file(input) {
        Ok(ast) => ast,
        Err(e) => {
            console_error(&format!("Error parsing file: {:?}", e));
            return "{\"error\": \"Parsing error\"}".to_string();
        }
    };
    let diagram = match parser::compile(&ast) {
        Ok(diagram) => diagram,
        Err(e) => {
            console_error(&format!("Error compiling file: {:?}", e));
            return "{\"error\": \"Compiling error\"}".to_string();
        }
    };
    let layout_result = layout_diagram(&diagram, &LayoutConfig::default());
    
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
    
    let output = DiagramOutput { groups, nodes, edges };
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
    
    let new_pos = parser::PointI { x, y };
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
    
    let new_pos = parser::PointI { x, y };
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
