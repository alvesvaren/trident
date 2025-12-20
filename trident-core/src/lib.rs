
use wasm_bindgen::prelude::*;
mod parser;
mod layout;
use layout::{layout_diagram, LayoutConfig, RectI};
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

/// The combined output sent to React
#[derive(Debug, Clone, Serialize)]
pub struct DiagramOutput {
    pub nodes: Vec<NodeOutput>,
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
    
    // Build a combined output for React
    let nodes: Vec<NodeOutput> = diagram.classes.iter().map(|c| {
        let bounds = layout.class_world_bounds.get(&c.cid).copied().unwrap_or(RectI { x: 0, y: 0, w: 0, h: 0 });
        NodeOutput {
            id: c.id.0.clone(),
            label: c.label.clone(),
            body_lines: c.body_lines.clone(),
            bounds,
        }
    }).collect();
    
    let output = DiagramOutput { nodes };
    to_string(&output).unwrap()
}
