//! Layout algorithms for Trident diagrams.
//!
//! This module contains all available layout algorithms:
//! - `graph_driven`: Default hierarchical layout that places connected nodes closer together
//! - `grid`: Simple left-to-right, top-to-bottom grid layout

mod graph_driven;
mod grid;

pub use graph_driven::{GraphDrivenLayout, layout_graph_driven};
pub use grid::{GridLayout, layout_grid};
