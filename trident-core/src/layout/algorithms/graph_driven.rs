//! Graph-driven hierarchical layout algorithm.
//!
//! This is the default layout algorithm that uses edge relationships
//! to place connected nodes closer together.

use std::collections::HashMap;
use crate::parser::{PointI, Diagram, GroupId, NodeId};
use crate::layout::{RectI, LayoutConfig, LayoutResult, LayoutStrategy};
use crate::layout::{post_order_groups, pre_order_groups, compute_group_local_bounds};
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
    // We'll fill these in for all nodes. Root is fixed at (0,0) local/world.
    let mut group_local_pos: HashMap<GroupId, PointI> = HashMap::new();
    let mut node_local_pos: HashMap<NodeId, PointI> = HashMap::new();

    group_local_pos.insert(diagram.root, PointI { x: 0, y: 0 });

    // Build adjacency from edges for graph-driven placement
    let adjacency = Adjacency::from_diagram(diagram);

    // Layout groups bottom-up (children first). Our compiler creates parents before children,
    // but for layout we want post-order traversal.
    let post = post_order_groups(diagram);

    // We also need per-group child sizes during packing.
    let mut group_local_bounds: HashMap<GroupId, RectI> = HashMap::new();

    // First pass: compute local positions for everything, and local bounds for groups.
    for gid in post {
        let g = &diagram.groups[gid.0];

        // Determine this group's own local position:
        // - If constrained (pos present), respect it.
        // - Else if root, it's already (0,0).
        // - Else default (0,0) for now (parent will place it).
        if gid != diagram.root {
            let p = g.pos.unwrap_or(PointI { x: 0, y: 0 });
            group_local_pos.insert(gid, p);
        }

        // Lay out children within this group using graph-driven placement.
        // Connected nodes will be placed closer together.
        layout_group_children_graph_driven(
            diagram,
            gid,
            cfg,
            &adjacency,
            &mut group_local_pos,
            &mut node_local_pos,
            &group_local_bounds, // contains bounds for child groups already
        );

        // After children placed, compute this group's local bounds (container box).
        let bounds = compute_group_local_bounds(
            diagram,
            gid,
            cfg,
            &group_local_pos,
            &node_local_pos,
            &group_local_bounds,
        );
        group_local_bounds.insert(gid, bounds);
    }

    // Second pass: accumulate world positions and compute world bounds.
    let mut group_world_pos: HashMap<GroupId, PointI> = HashMap::new();
    let mut node_world_pos: HashMap<NodeId, PointI> = HashMap::new();
    let mut group_world_bounds: HashMap<GroupId, RectI> = HashMap::new();
    let mut node_world_bounds: HashMap<NodeId, RectI> = HashMap::new();

    group_world_pos.insert(diagram.root, PointI { x: 0, y: 0 });

    // Traverse groups in pre-order so parents have world pos before children.
    let pre = pre_order_groups(diagram);

    for gid in pre {
        let g_local = *group_local_pos.get(&gid).unwrap_or(&PointI { x: 0, y: 0 });
        let g_world = if gid == diagram.root {
            PointI { x: 0, y: 0 }
        } else {
            let parent = diagram.groups[gid.0].parent.expect("non-root group must have parent");
            let pw = *group_world_pos.get(&parent).unwrap();
            PointI { x: pw.x + g_local.x, y: pw.y + g_local.y }
        };

        group_world_pos.insert(gid, g_world);

        // Convert local bounds -> world bounds
        let lb = *group_local_bounds.get(&gid).unwrap();
        let wb = RectI { x: g_world.x + lb.x, y: g_world.y + lb.y, w: lb.w, h: lb.h };
        group_world_bounds.insert(gid, wb);

        // Nodes directly in this group
        for &nid in &diagram.groups[gid.0].children_nodes {
            let n_local = *node_local_pos.get(&nid).unwrap_or(&PointI { x: 0, y: 0 });
            let n_world = PointI { x: g_world.x + n_local.x, y: g_world.y + n_local.y };
            node_world_pos.insert(nid, n_world);

            let node = &diagram.nodes[nid.0];
            let sz = crate::layout::get_node_size(node, cfg);
            node_world_bounds.insert(nid, RectI { x: n_world.x, y: n_world.y, w: sz.w, h: sz.h });
        }
    }

    LayoutResult {
        group_local_pos,
        node_local_pos,
        group_world_pos,
        node_world_pos,
        group_world_bounds,
        node_world_bounds,
    }
}
