//! Trident Core - UML Diagram Language Compiler
//!
//! This crate provides parsing, compilation, and layout for the Trident diagram language.

mod parser;
mod layout;
mod output;
mod wasm;

// Re-export for external use
pub use output::*;
pub use wasm::*;
pub use layout::{LayoutConfig, LayoutResult, RectI, SizeI};
pub use parser::{PointI, Diagram, GroupId, NodeId};
