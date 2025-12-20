
use wasm_bindgen::prelude::*;
mod parser;
mod layout;
use layout::{layout_diagram, LayoutConfig, RectI};
use parser::Arrow;
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
    pub label: Option<String>,
    pub body_lines: Vec<String>,
    pub bounds: RectI,
    /// Whether this node has a fixed position (@pos in the source)
    pub has_pos: bool,
}

/// An edge between two nodes
#[derive(Debug, Clone, Serialize)]
pub struct EdgeOutput {
    pub from: String,
    pub to: String,
    pub arrow: String,
    pub label: Option<String>,
}

/// A group container
#[derive(Debug, Clone, Serialize)]
pub struct GroupOutput {
    pub id: String,
    pub bounds: RectI,
}

fn arrow_to_string(arrow: Arrow) -> String {
    match arrow {
        Arrow::ExtendsLeft => "extends_left".to_string(),
        Arrow::ExtendsRight => "extends_right".to_string(),
        Arrow::Aggregate => "aggregate".to_string(),
        Arrow::Compose => "compose".to_string(),
        Arrow::AssocRight => "assoc_right".to_string(),
        Arrow::AssocLeft => "assoc_left".to_string(),
        Arrow::DepRight => "dep_right".to_string(),
        Arrow::DepLeft => "dep_left".to_string(),
        Arrow::Line => "line".to_string(),
        Arrow::Dotted => "dotted".to_string(),
    }
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
    let layout = layout_diagram(&diagram, &LayoutConfig::default());
    
    // Build groups (only named groups, skip root and anonymous)
    let groups: Vec<GroupOutput> = diagram.groups.iter()
        .filter(|g| g.id.is_some() && g.gid != diagram.root)
        .filter_map(|g| {
            let bounds = layout.group_world_bounds.get(&g.gid).copied()?;
            Some(GroupOutput {
                id: g.id.as_ref()?.0.clone(),
                bounds,
            })
        })
        .collect();
    
    // Build nodes
    let nodes: Vec<NodeOutput> = diagram.classes.iter().map(|c| {
        let bounds = layout.class_world_bounds.get(&c.cid).copied().unwrap_or(RectI { x: 0, y: 0, w: 0, h: 0 });
        NodeOutput {
            id: c.id.0.clone(),
            label: c.label.clone(),
            body_lines: c.body_lines.clone(),
            bounds,
            has_pos: c.pos.is_some(),
        }
    }).collect();
    
    // Build edges
    let edges: Vec<EdgeOutput> = diagram.edges.iter().map(|e| {
        let from_id = diagram.classes[e.from.0].id.0.clone();
        let to_id = diagram.classes[e.to.0].id.0.clone();
        EdgeOutput {
            from: from_id,
            to: to_id,
            arrow: arrow_to_string(e.arrow),
            label: e.label.clone(),
        }
    }).collect();
    
    let output = DiagramOutput { groups, nodes, edges };
    to_string(&output).unwrap()
}

/// Update a class position and return the new source code
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
    if parser::update_class_position(&mut ast, class_id, new_pos) {
        parser::emit_file(&ast)
    } else {
        console_error(&format!("Class '{}' not found", class_id));
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

/// Remove a class position (unlock it for auto-layout) and return the new source code
#[wasm_bindgen]
pub fn remove_class_pos(source: &str, class_id: &str) -> String {
    let mut ast = match parser::parse_file(source) {
        Ok(ast) => ast,
        Err(e) => {
            console_error(&format!("Error parsing file: {:?}", e));
            return source.to_string();
        }
    };
    
    if parser::remove_class_position(&mut ast, class_id) {
        parser::emit_file(&ast)
    } else {
        console_error(&format!("Class '{}' not found", class_id));
        source.to_string()
    }
}

/// Remove all positions from all classes and groups (unlock everything)
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
