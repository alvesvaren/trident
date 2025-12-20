
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
}

/// An edge between two nodes
#[derive(Debug, Clone, Serialize)]
pub struct EdgeOutput {
    pub from: String,
    pub to: String,
    pub arrow: String,
    pub label: Option<String>,
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
    
    // Build nodes
    let nodes: Vec<NodeOutput> = diagram.classes.iter().map(|c| {
        let bounds = layout.class_world_bounds.get(&c.cid).copied().unwrap_or(RectI { x: 0, y: 0, w: 0, h: 0 });
        NodeOutput {
            id: c.id.0.clone(),
            label: c.label.clone(),
            body_lines: c.body_lines.clone(),
            bounds,
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
    
    let output = DiagramOutput { nodes, edges };
    to_string(&output).unwrap()
}
