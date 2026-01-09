//! Output types for React frontend consumption.
//!
//! These structs are serialized to JSON and sent to the React frontend
//! for rendering the diagram.

use crate::layout::{RectI, NodeRenderingConfig};
use crate::parser::PointI;
use serde::Serialize;

/// Type of text element for rendering
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum TextElement {
    /// Stereotype line (modifiers and kind)
    Stereotype { text: String, y: i32, font_size: i32 },
    /// Title line (node label/id)
    Title { text: String, y: i32, font_size: i32, italic: bool },
    /// Separator line (---)
    Separator { x1: i32, y1: i32, x2: i32, y2: i32 },
    /// Regular body text line
    BodyText { text: String, y: i32, font_size: i32 },
}

/// A rendered node ready for React to display
#[derive(Debug, Clone, Serialize)]
pub struct NodeOutput {
    pub id: String,
    /// Node kind: "class" or "node"
    pub kind: String,
    /// Modifiers: "abstract", "interface", "enum", "rectangle", "circle", "diamond", etc.
    pub modifiers: Vec<String>,
    pub label: Option<String>,
    /// Structured text elements with calculated positions
    pub text_elements: Vec<TextElement>,
    /// Rendering constants for React to use
    pub rendering_config: NodeRenderingConfig,
    pub bounds: RectI,
    /// Whether this node has a fixed position (@pos in the source)
    pub has_pos: bool,
    /// World position of parent group (for calculating local coords during drag)
    pub parent_offset: PointI,
    /// Whether this node was explicitly declared (false for implicit nodes from relations)
    pub explicit: bool,
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

/// Error information for Monaco editor markers
#[derive(Debug, Clone, Serialize)]
pub struct ErrorInfo {
    pub message: String,
    pub line: usize,      // 1-based line number
    pub column: usize,    // 1-based column number
    pub end_line: usize,  // 1-based end line (same as line for single-line errors)
    pub end_column: usize, // 1-based end column
}

/// The combined output sent to React
#[derive(Debug, Clone, Serialize)]
pub struct DiagramOutput {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<GroupOutput>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<NodeOutput>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<EdgeOutput>,
    /// List of implicit node IDs (for editor info diagnostics)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub implicit_nodes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}
