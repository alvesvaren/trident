//! Layout algorithms for Trident diagrams.
//!
//! This module contains all available layout algorithms:
//! - `graph_driven`: Default hierarchical layout that places connected nodes closer together
//! - `grid`: Simple left-to-right, top-to-bottom grid layout
//! - `constrained`: Constraint-based layout with barycenter positioning
//! - `radial`: Radial tree layout (mind-map style)

mod graph_driven;
mod grid;
mod constrained;
mod radial;

pub use graph_driven::{GraphDrivenLayout, layout_graph_driven};
pub use grid::{GridLayout, layout_grid};
pub use constrained::{ConstrainedLayout, layout_constrained};
pub use radial::{RadialLayout, layout_radial};
