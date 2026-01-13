//! Constrained layout algorithm.
//!
//! This layout uses constraint-based placement with barycenter positioning.
//! It respects @pos constraints and places connected nodes close together.

use std::collections::HashMap;
use crate::parser::{PointI, Diagram, GroupId, NodeId};
use crate::layout::{RectI, LayoutConfig, LayoutResult, LayoutStrategy};
use crate::layout::{post_order_groups, pre_order_groups, compute_group_local_bounds, get_node_size};
use crate::layout::adjacency::Adjacency;
use crate::layout::force_placement::layout_nodes_constrained;

/// Constrained layout implementation.
/// Places connected nodes closer together using barycenter positioning.
pub struct ConstrainedLayout;

impl LayoutStrategy for ConstrainedLayout {
    fn layout(&self, diagram: &Diagram, cfg: &LayoutConfig) -> LayoutResult {
        layout_constrained(diagram, cfg)
    }
}

/// Layout children of a group using constrained placement.
fn layout_group_children_constrained(
    diagram: &Diagram,
    gid: GroupId,
    cfg: &LayoutConfig,
    adjacency: &Adjacency,
    group_local_pos: &mut HashMap<GroupId, PointI>,
    node_local_pos: &mut HashMap<NodeId, PointI>,
    group_local_bounds: &HashMap<GroupId, RectI>,
) {
    let g = &diagram.groups[gid.0];

    // 1. Separate fixed and free items
    let mut fixed_groups: Vec<GroupId> = Vec::new();
    let mut free_groups: Vec<GroupId> = Vec::new();
    let mut fixed_nodes: Vec<NodeId> = Vec::new();
    let mut free_nodes: Vec<NodeId> = Vec::new();

    for &cgid in &g.children_groups {
        if diagram.groups[cgid.0].pos.is_some() {
            fixed_groups.push(cgid);
        } else {
            free_groups.push(cgid);
        }
    }

    for &nid in &g.children_nodes {
        if diagram.nodes[nid.0].pos.is_some() {
            fixed_nodes.push(nid);
        } else {
            free_nodes.push(nid);
        }
    }

    // 2. Assign fixed positions
    for cgid in &fixed_groups {
        let p = diagram.groups[cgid.0].pos.unwrap();
        group_local_pos.insert(*cgid, p);
    }
    for nid in &fixed_nodes {
        let p = diagram.nodes[nid.0].pos.unwrap();
        node_local_pos.insert(*nid, p);
    }

    // 3. Place free groups using simple sequential layout
    // (Groups are less common; we use a simpler approach here)
    if !free_groups.is_empty() {
        let mut next_x = cfg.group_padding;
        let mut next_y = cfg.group_padding;
        let mut row_max_h = 0i32;
        
        for cgid in &free_groups {
            let bounds = group_local_bounds.get(cgid).copied().unwrap_or(RectI {
                x: 0, y: 0, w: cfg.min_group_size.w, h: cfg.min_group_size.h,
            });
            
            // Check wrapping
            if next_x + bounds.w > cfg.max_row_w && next_x > cfg.group_padding {
                next_x = cfg.group_padding;
                next_y += row_max_h + cfg.gap;
                row_max_h = 0;
            }
            
            group_local_pos.insert(*cgid, PointI { x: next_x, y: next_y });
            next_x += bounds.w + cfg.gap;
            row_max_h = row_max_h.max(bounds.h);
        }
    }

    // 4. Place free nodes using constrained layout
    if !free_nodes.is_empty() {
        layout_nodes_constrained(
            diagram,
            &free_nodes,
            &fixed_nodes,
            cfg,
            adjacency,
            node_local_pos,
        );
    }
}

/// Internal implementation of the constrained layout.
pub fn layout_constrained(diagram: &Diagram, cfg: &LayoutConfig) -> LayoutResult {
    let mut group_local_pos: HashMap<GroupId, PointI> = HashMap::new();
    let mut node_local_pos: HashMap<NodeId, PointI> = HashMap::new();

    group_local_pos.insert(diagram.root, PointI { x: 0, y: 0 });

    // Build adjacency from edges
    let adjacency = Adjacency::from_diagram(diagram);

    // Layout groups bottom-up (children first)
    let post = post_order_groups(diagram);
    let mut group_local_bounds: HashMap<GroupId, RectI> = HashMap::new();

    for gid in post {
        let g = &diagram.groups[gid.0];

        // Determine this group's own local position
        if gid != diagram.root {
            let p = g.pos.unwrap_or(PointI { x: 0, y: 0 });
            group_local_pos.insert(gid, p);
        }

        // Lay out children within this group
        layout_group_children_constrained(
            diagram,
            gid,
            cfg,
            &adjacency,
            &mut group_local_pos,
            &mut node_local_pos,
            &group_local_bounds,
        );

        // After children placed, compute this group's local bounds
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

    // Second pass: accumulate world positions
    let mut group_world_pos: HashMap<GroupId, PointI> = HashMap::new();
    let mut node_world_pos: HashMap<NodeId, PointI> = HashMap::new();
    let mut group_world_bounds: HashMap<GroupId, RectI> = HashMap::new();
    let mut node_world_bounds: HashMap<NodeId, RectI> = HashMap::new();

    group_world_pos.insert(diagram.root, PointI { x: 0, y: 0 });

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
            let sz = get_node_size(node, cfg);
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
