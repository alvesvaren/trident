
use wasm_bindgen::prelude::*;
mod parser;
mod layout;
use layout::{layout_diagram, LayoutConfig};
use serde_json::to_string;

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
            return "{\"error\": \"Parsing error\"}".to_string();
        }
    };
    let diagram = match parser::compile(&ast) {
        Ok(ast) => ast,
        Err(e) => {
            console_error(&format!("Error compiling file: {:?}", e));
            return "{\"error\": \"Compiling error\"}".to_string();
        }
    };
    let layout = layout_diagram(&diagram, &LayoutConfig::default());
    return to_string(&layout).unwrap().to_string();
}
