//! Layout algorithms for Trident diagrams.
//!
//! This module contains all available layout algorithms:
//! - `graph_driven`: Default hierarchical layout that places connected nodes closer together
//! - `grid`: Simple left-to-right, top-to-bottom grid layout
//! - `organic`: Stress-based layout with crossing reduction, for graphs without a top-down flow

mod graph_driven;
mod grid;
mod organic;

pub use graph_driven::{GraphDrivenLayout, layout_graph_driven};
pub use grid::{GridLayout, layout_grid};
pub use organic::{OrganicLayout, layout_organic};
