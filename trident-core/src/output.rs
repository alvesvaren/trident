//! Output types for React frontend consumption.
//!
//! These structs are serialized to JSON and sent to the React frontend
//! for rendering the diagram.

use crate::layout::RectI;
use crate::parser::PointI;
use serde::Serialize;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}
