//! Graph-driven hierarchical layout algorithm.
//!
//! This is the default layout algorithm that uses edge relationships
//! to place connected nodes closer together.

use crate::parser::Diagram;
use crate::layout::{LayoutConfig, LayoutResult, LayoutStrategy, layout_nested};
use crate::layout::adjacency::Adjacency;
use crate::layout::placement::layout_group_children_graph_driven;

/// Graph-driven hierarchical layout implementation.
/// Places connected nodes closer together based on edge relationships.
pub struct GraphDrivenLayout;

impl LayoutStrategy for GraphDrivenLayout {
    fn layout(&self, diagram: &Diagram, cfg: &LayoutConfig) -> LayoutResult {
        layout_graph_driven(diagram, cfg)
    }
}

/// Internal implementation of the graph-driven layout.
pub fn layout_graph_driven(diagram: &Diagram, cfg: &LayoutConfig) -> LayoutResult {
    // Build adjacency from edges for graph-driven placement
    let adjacency = Adjacency::from_diagram(diagram);

    layout_nested(diagram, cfg, |gid, group_local_pos, node_local_pos, group_local_bounds| {
        // Connected nodes will be placed closer together.
        layout_group_children_graph_driven(
            diagram,
            gid,
            cfg,
            &adjacency,
            group_local_pos,
            node_local_pos,
            group_local_bounds, // contains bounds for child groups already
        );
    })
}
